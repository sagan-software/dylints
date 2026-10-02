#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::string_slice,
    clippy::unwrap_used,
    clippy::let_underscore_must_use,
    reason = "rustc supplies validated AST variants and UTF-8 source offsets to this lint"
)]

//! A lint to check for manual thiserror source display forwarding.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

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
    pub THISERROR_TRANSPARENT_SINGLE_SOURCE_DISPLAY,
    Warn,
    "`thiserror` wrapper manually forwards source display",
    ThiserrorTransparentSingleSourceDisplay
}

impl EarlyLintPass for ThiserrorTransparentSingleSourceDisplay {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let derive_names = thiserror_derive_names(krate);

        for item in &krate.items {
            check_item(cx, item, &derive_names);
        }
    }
}

/// Check one AST item and recursively visit loaded modules for thiserror wrappers.
fn check_item(cx: &EarlyContext<'_>, item: &ast::Item, derive_names: &BTreeSet<Symbol>) {
    // Check display attributes only on structs and enums owned by thiserror.
    match &item.kind {
        ast::ItemKind::Struct(_, _, data)
            if item_derives_thiserror_error(cx, item, derive_names) =>
        {
            check_error_attrs(cx, &item.attrs, data.fields());
        }
        ast::ItemKind::Enum(_, _, enum_def)
            if item_derives_thiserror_error(cx, item, derive_names) =>
        {
            for variant in &enum_def.variants {
                check_error_attrs(cx, &variant.attrs, variant.data.fields());
            }
        }
        ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) => {
            // Recurse through loaded inline modules to preserve crate-wide coverage.
            for item in items {
                check_item(cx, item, derive_names);
            }
        }
        _ => {}
    }
}

/// Check error attributes whose format string only forwards the single source field.
fn check_error_attrs(cx: &EarlyContext<'_>, attrs: &[Attribute], fields: &[ast::FieldDef]) {
    // Parse only thiserror display attributes with a literal format string.
    for attr in attrs.iter().filter(|attr| attr.has_name(symbol("error"))) {
        let Some(format) = error_format_literal(attr) else {
            continue;
        };
        if !matches!(format.as_str(), "{0}" | "{source}")
            || !looks_like_single_source_wrapper(cx, fields)
        {
            continue;
        }

        // Replace the exact forwarding attribute while retaining its source field.
        emit_span_lint_with_suggestion(
            cx,
            THISERROR_TRANSPARENT_SINGLE_SOURCE_DISPLAY,
            attr.span,
            "`thiserror` can forward this source transparently",
            "replace the manual display forwarding with `#[error(transparent)]`",
            "#[error(transparent)]",
        );
    }
}

/// Return whether a variant or struct has exactly one field that thiserror
/// treats as a source.
fn looks_like_single_source_wrapper(cx: &EarlyContext<'_>, fields: &[ast::FieldDef]) -> bool {
    let [field] = fields else {
        return false;
    };

    // Keep the fix exact: one visible source field and no other payload.
    has_attr(&field.attrs, "from")
        || has_attr(&field.attrs, "source")
        || field_name_is(cx, field, "source")
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
    match &item.kind {
        ast::ItemKind::Use(tree) => collect_use_tree_aliases(&[], tree, aliases),
        ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) => {
            for item in items {
                collect_thiserror_derive_names(item, aliases);
            }
        }
        _ => {}
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

    match &tree.kind {
        // Record direct imports and explicit aliases of the target derive.
        UseTreeKind::Simple(rename) if path == [symbol("thiserror"), symbol("Error")] => {
            let _ =
                aliases.insert(rename.map_or_else(|| *path.last().unwrap(), |ident| ident.name));
        }
        UseTreeKind::Nested { items, .. } => {
            for (nested, _) in items {
                collect_use_tree_aliases(&path, nested, aliases);
            }
        }
        _ => {}
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
    source_derives_thiserror_error(&source[window_start..item_start], derive_names)
}

/// Return whether raw source contains a derive attribute for `thiserror::Error`.
fn source_derives_thiserror_error(source: &str, derive_names: &BTreeSet<Symbol>) -> bool {
    attr_ranges(source, "derive")
        .into_iter()
        .any(|(start, end)| {
            source[start..end]
                .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
                .any(|segment| {
                    segment == "thiserror::Error"
                        || derive_names
                            .iter()
                            .any(|derive_name| segment == derive_name.as_str())
                })
        })
}

/// Return byte ranges for attributes matching the target name.
fn attr_ranges(source: &str, name: &str) -> Vec<(usize, usize)> {
    // Scan forward through complete outer attributes in source order.
    let mut ranges = Vec::new();
    let mut search_start = 0;

    while let Some(relative_start) = source[search_start..].find("#[") {
        // Convert the relative match and closing bracket into absolute offsets.
        let start = search_start + relative_start;
        let Some(end) = source[start..]
            .find(']')
            .map(|relative_end| start + relative_end + 1)
        else {
            break;
        };

        // Retain only the simple or list-form attribute name.
        let body = source[start + 2..end - 1].trim();
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

/// Extract the first literal argument from a `#[error("...")]` attribute.
fn error_format_literal(attr: &Attribute) -> Option<String> {
    let items = attr.meta_item_list()?;
    let MetaItemInner::Lit(lit) = items.first()? else {
        return None;
    };

    lit.value_as_str().map(|value| value.to_string())
}

/// Find the first attribute with the simple name.
fn find_attr<'attrs>(attrs: &'attrs [Attribute], name: &str) -> Option<&'attrs Attribute> {
    attrs.iter().find(|attr| attr.has_name(symbol(name)))
}

/// Return whether an attribute list contains the simple name.
fn has_attr(attrs: &[Attribute], name: &str) -> bool {
    find_attr(attrs, name).is_some()
}

/// Intern an attribute or path segment name for rustc AST comparisons.
fn symbol(name: &str) -> Symbol {
    Symbol::intern(name)
}

/// Emit a machine-applicable replacement for the redundant display-forwarding attribute.
fn emit_span_lint_with_suggestion(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: &'static str,
) {
    // Replacing one complete thiserror attribute is an exact local edit.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(message);
            let _ = diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
