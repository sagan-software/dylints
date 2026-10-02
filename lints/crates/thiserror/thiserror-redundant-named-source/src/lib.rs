#![feature(rustc_private)]

//! A lint to check for redundant source attributes on source fields.
//!
//! This Dylint library resolves thiserror source attributes, reports redundant
//! markers, and recommends retaining only the field behavior that is intended.
//!
//! The README defines the supported source shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use std::collections::BTreeSet;

use rustc_ast::{
    Crate,
    ast::{self, Attribute, MetaItem, MetaItemInner, ModKind, UseTree, UseTreeKind},
};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::{Span, symbol::Symbol};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_REDUNDANT_NAMED_SOURCE,
    Warn,
    "`thiserror` source field has a redundant source attribute",
    ThiserrorRedundantNamedSource
}

impl EarlyLintPass for ThiserrorRedundantNamedSource {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let derive_names = thiserror_derive_names(krate);

        for item in &krate.items {
            check_item(cx, item, &derive_names);
        }
    }
}

/// Check one AST item and recursively visit loaded modules for thiserror-owned fields.
fn check_item(cx: &EarlyContext<'_>, item: &ast::Item, derive_names: &BTreeSet<Symbol>) {
    // Check fields only on structs and enums owned by thiserror.
    if let ast::ItemKind::Struct(_, _, data) = &item.kind
        && item_derives_thiserror_error(cx, item, derive_names)
    {
        check_fields(cx, data.fields());
        return;
    }
    // Check every variant field when the owning enum derives thiserror.
    if let ast::ItemKind::Enum(_, _, enum_def) = &item.kind
        && item_derives_thiserror_error(cx, item, derive_names)
    {
        for variant in &enum_def.variants {
            check_fields(cx, variant.data.fields());
        }
        return;
    }
    if let ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) = &item.kind {
        // Recurse through loaded inline modules to preserve crate-wide coverage.
        for item in items {
            check_item(cx, item, derive_names);
        }
    }
}

/// Check named `source` fields for redundant explicit `#[source]` attributes.
fn check_fields(cx: &EarlyContext<'_>, fields: &[ast::FieldDef]) {
    // Find explicit source attributes before checking the inferred field name.
    for field in fields {
        let Some(source_attr) = find_attr(&field.attrs, "source") else {
            continue;
        };
        if !field_name_is(cx, field, "source") {
            continue;
        }

        // Delete only the redundant attribute and retain the source field.
        emit_span_lint_with_suggestion(
            cx,
            THISERROR_REDUNDANT_NAMED_SOURCE,
            source_attr.span,
            "`source` fields are source errors without an attribute",
            "remove the redundant `#[source]` attribute",
            "",
        );
    }
}

/// Return whether a named field spells the expected identifier without raw syntax.
fn field_name_is(cx: &EarlyContext<'_>, field: &ast::FieldDef, expected: &str) -> bool {
    let Some(ident) = field.ident else {
        return false;
    };
    if ident.name != symbol(expected) {
        return false;
    }

    // Raw identifiers such as `r#source` are distinct fields for thiserror's source inference.
    cx.sess()
        .source_map()
        .span_to_snippet(ident.span)
        .is_ok_and(|snippet| snippet == expected)
}

/// Collect AST-level aliases for `thiserror::Error` imports in the crate.
fn thiserror_derive_names(krate: &Crate) -> BTreeSet<Symbol> {
    // Gather every crate-local alias that can name `thiserror::Error`.
    let mut aliases = BTreeSet::new();

    for item in &krate.items {
        collect_thiserror_derive_names(item, &mut aliases);
    }

    aliases
}

/// Recursively collect `thiserror::Error` aliases from use items.
fn collect_thiserror_derive_names(item: &ast::Item, aliases: &mut BTreeSet<Symbol>) {
    // Record aliases introduced by this item before visiting nested modules.
    if let ast::ItemKind::Use(tree) = &item.kind {
        collect_use_tree_aliases(&[], tree, aliases);
    }
    // Recurse only into modules whose items are loaded in this AST.
    if let ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) = &item.kind {
        for item in items {
            collect_thiserror_derive_names(item, aliases);
        }
    }
}

/// Collect aliases from one possibly nested use tree.
fn collect_use_tree_aliases(prefix: &[Symbol], tree: &UseTree, aliases: &mut BTreeSet<Symbol>) {
    // Extend the inherited path with this use-tree prefix.
    let mut path = prefix.to_vec();
    path.extend(
        tree.prefix
            .segments
            .iter()
            .map(|segment| segment.ident.name),
    );

    // Record direct imports and explicit aliases of the target derive.
    if let UseTreeKind::Simple(rename) = &tree.kind
        && path == [symbol("thiserror"), symbol("Error")]
        && let Some(last) = path.last()
    {
        let _ = aliases.insert(rename.map_or(*last, |ident| ident.name));
    }
    if let UseTreeKind::Nested { items, .. } = &tree.kind {
        for (nested, _) in items {
            collect_use_tree_aliases(&path, nested, aliases);
        }
    }
}

/// Return whether an item derives `thiserror::Error` in AST or nearby source text.
fn item_derives_thiserror_error(
    cx: &EarlyContext<'_>,
    item: &ast::Item,
    derive_names: &BTreeSet<Symbol>,
) -> bool {
    if derives_thiserror_error(&item.attrs, derive_names) {
        return true;
    }

    item_source_derives_thiserror_error(cx, item, derive_names)
}

/// Return whether an AST attribute list contains a `thiserror::Error` derive.
fn derives_thiserror_error(attrs: &[Attribute], derive_names: &BTreeSet<Symbol>) -> bool {
    // Helper attributes are only thiserror-owned when the item actually derives thiserror::Error.
    attrs.iter().any(|attr| {
        attr.has_name(symbol("derive"))
            && attr.meta_item_list().is_some_and(|items| {
                items.iter().any(|item| match item {
                    MetaItemInner::MetaItem(meta) => is_thiserror_error_derive(meta, derive_names),
                    MetaItemInner::Lit(_) => false,
                })
            })
    })
}

/// Check a small source window before an item for derive syntax lost after expansion.
fn item_source_derives_thiserror_error(
    cx: &EarlyContext<'_>,
    item: &ast::Item,
    derive_names: &BTreeSet<Symbol>,
) -> bool {
    // Recover the original source and translate the item span into a file offset.
    let source_map = cx.sess().source_map();
    let source_file = source_map.lookup_source_file(item.span.lo());
    let Some(source) = source_file.src.as_deref() else {
        return false;
    };
    let Ok(item_start) = usize::try_from((item.span.lo() - source_file.start_pos).0) else {
        return false;
    };
    if item_start > source.len() {
        return false;
    }

    // Keep the source fallback local to the attribute region before this item.
    let mut window_start = item_start.saturating_sub(2048);
    // Advance to a UTF-8 boundary before slicing the source window.
    while !source.is_char_boundary(window_start) {
        window_start += 1;
    }
    source
        .get(window_start..item_start)
        .is_some_and(|window| source_derives_thiserror_error(window, derive_names))
}

/// Return whether raw source contains a derive attribute for `thiserror::Error`.
fn source_derives_thiserror_error(source: &str, derive_names: &BTreeSet<Symbol>) -> bool {
    attr_ranges(source, "derive")
        .into_iter()
        .any(|(start, end)| {
            source.get(start..end).is_some_and(|attribute| {
                attribute
                    .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
                    .any(|segment| {
                        segment == "thiserror::Error"
                            || derive_names
                                .iter()
                                .any(|derive_name| segment == derive_name.as_str())
                    })
            })
        })
}

/// Return byte ranges for attributes matching the target name.
fn attr_ranges(source: &str, name: &str) -> Vec<(usize, usize)> {
    // Scan forward through complete outer attributes in source order.
    let mut ranges = Vec::new();
    let mut search_start = 0;

    while let Some(relative_start) = source.get(search_start..).and_then(|tail| tail.find("#[")) {
        // Convert the relative match and closing bracket into absolute offsets.
        let start = search_start + relative_start;
        let Some(end) = source
            .get(start..)
            .and_then(|attribute| attribute.find(']'))
            .map(|relative_end| start + relative_end + 1)
        else {
            break;
        };

        // Retain only the simple or list-form attribute name.
        let body = source
            .get(start + 2..end - 1)
            .map(str::trim)
            .unwrap_or_default();
        if body == name || body.starts_with(&format!("{name}(")) {
            ranges.push((start, end));
        }
        // Resume after the complete attribute to guarantee forward progress.
        search_start = end;
    }

    ranges
}

/// Return whether one derive meta item names `thiserror::Error`.
fn is_thiserror_error_derive(meta: &MetaItem, derive_names: &BTreeSet<Symbol>) -> bool {
    let segments: Vec<_> = meta
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.name)
        .collect();

    // Accept the qualified derive path and local aliases imported from `thiserror::Error`.
    segments == [symbol("thiserror"), symbol("Error")]
        || segments
            .first()
            .is_some_and(|name| segments.len() == 1 && derive_names.contains(name))
}

/// Find the first attribute with the simple name.
fn find_attr<'attrs>(attrs: &'attrs [Attribute], name: &str) -> Option<&'attrs Attribute> {
    attrs.iter().find(|attr| attr.has_name(symbol(name)))
}

/// Intern an attribute or path segment name for rustc AST comparisons.
fn symbol(name: &str) -> Symbol {
    Symbol::intern(name)
}

/// Emit a machine-applicable deletion for the redundant source attribute.
fn emit_span_lint_with_suggestion(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: &'static str,
) {
    // The replacement deletes only the redundant attribute, so it is safe for `--fix`.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_suggestion =
                diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
