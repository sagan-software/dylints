#![feature(rustc_private)]

//! A lint to check for raw identifiers in thiserror format strings.
//!
//! This Dylint library resolves thiserror attributes, reports raw field names
//! in format strings, and recommends the unraw field spelling.
//!
//! The README defines the supported source shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use rustc_ast::{
    Crate,
    ast::{self, Attribute, MetaItem, MetaItemInner, ModKind, UseTree, UseTreeKind},
};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext, symbol::Symbol};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_RAW_FIELD_FORMAT,
    Warn,
    "`thiserror` format string uses a raw field identifier",
    ThiserrorRawFieldFormat
}

impl EarlyLintPass for ThiserrorRawFieldFormat {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Check cfg-active AST items with resolved derive aliases first.
        let derive_names = thiserror_derive_names(krate);

        for item in &krate.items {
            check_item(cx, item, &derive_names);
        }

        // Scan loaded source afterward to cover cfg-disabled attributes.
        for candidate in loaded_rust_sources(cx) {
            check_source_fallback(cx, &candidate);
        }
    }
}

/// Check one AST item and recursively visit loaded modules for `thiserror::Error` types.
fn check_item(cx: &EarlyContext<'_>, item: &ast::Item, derive_names: &BTreeSet<Symbol>) {
    // Inspect struct-level and field-level formats on thiserror structs.
    if let ast::ItemKind::Struct(_, _, data) = &item.kind
        && derives_thiserror_error(&item.attrs, derive_names)
    {
        check_error_attrs(cx, &item.attrs);
        check_fields(cx, data.fields());
        return;
    }
    if let ast::ItemKind::Enum(_, _, enum_def) = &item.kind
        && derives_thiserror_error(&item.attrs, derive_names)
    {
        // Inspect the enum and every variant and field format it owns.
        check_error_attrs(cx, &item.attrs);
        for variant in &enum_def.variants {
            check_error_attrs(cx, &variant.attrs);
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

/// Check field-level `#[error(...)]` attributes on enum or struct fields.
fn check_fields(cx: &EarlyContext<'_>, fields: &[ast::FieldDef]) {
    for field in fields {
        check_error_attrs(cx, &field.attrs);
    }
}

/// Check error attributes parsed by rustc's AST for raw field placeholders.
fn check_error_attrs(cx: &EarlyContext<'_>, attrs: &[Attribute]) {
    // Parse only thiserror display attributes with a raw field placeholder.
    for attr in attrs.iter().filter(|attr| attr.has_name(symbol("error"))) {
        let Some(format) = error_format_literal(attr) else {
            continue;
        };
        if !format.contains("{r#") {
            continue;
        }
        let Some(suggestion) =
            attr_snippet(cx, attr.span).map(|snippet| snippet.replace("{r#", "{"))
        else {
            continue;
        };

        // Replace only the raw marker inside the original attribute snippet.
        emit_span_lint_with_suggestion(
            cx,
            THISERROR_RAW_FIELD_FORMAT,
            attr.span,
            "`thiserror` 2 expects unraw field names in format strings",
            "remove `r#` inside the format placeholder",
            suggestion,
        );
    }
}

/// Source file loaded for the text fallback that sees cfg-disabled code.
struct SourceCandidate {
    /// Full source text for the candidate Rust file.
    source: String,
    /// Byte position where this source file starts in rustc's source map.
    start_pos: BytePos,
}

/// Check raw source text for derive and error attributes that the AST path may miss.
fn check_source_fallback(cx: &EarlyContext<'_>, candidate: &SourceCandidate) {
    // Establish thiserror ownership from qualified derives or local aliases.
    let derive_names = source_thiserror_derive_names(&candidate.source);
    if !source_derives_thiserror_error(&candidate.source, &derive_names) {
        return;
    }

    for (start, end) in attr_ranges(&candidate.source, "error") {
        // Retain only literal formats containing a raw field placeholder.
        let Some(attr) = candidate.source.get(start..end) else {
            continue;
        };
        if !error_format_literal_from_source(attr).is_some_and(|format| format.contains("{r#")) {
            continue;
        }

        emit_span_lint_with_suggestion(
            cx,
            THISERROR_RAW_FIELD_FORMAT,
            range_span(candidate.start_pos, start, end),
            "`thiserror` 2 expects unraw field names in format strings",
            "remove `r#` inside the format placeholder",
            attr.replace("{r#", "{"),
        );
    }
}

/// Load user Rust source files from rustc's source map for fallback scanning.
fn loaded_rust_sources(cx: &EarlyContext<'_>) -> Vec<SourceCandidate> {
    // Deduplicate local Rust files exposed through rustc's source map.
    let files = cx.sess().source_map().files();
    let mut seen = BTreeSet::new();
    let mut candidates = Vec::new();

    for source_file in files.iter() {
        // Exclude repeated paths, non-Rust files, and dependency sources.
        let Some(path) = source_file_path(source_file) else {
            continue;
        };
        if !seen.insert(path.clone()) || !is_rust_file(&path) || is_dependency_source(&path) {
            continue;
        }

        let Some(source) = read_file(&path) else {
            continue;
        };

        // Preserve the source-map origin for diagnostic span reconstruction.
        candidates.push(SourceCandidate {
            source,
            start_pos: source_file.start_pos,
        });
    }

    drop(files);

    candidates
}

/// Convert a rustc source-file name into a local filesystem path.
fn source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    source_file.name.clone().into_local_path()
}

/// Return whether a path points at a Rust source file.
fn is_rust_file(path: &Path) -> bool {
    path.extension().is_some_and(|ext| ext == "rs")
}

/// Return whether a path belongs to dependency or compiler source that should be skipped.
fn is_dependency_source(path: &Path) -> bool {
    has_path_component(path, "rustc")
        || (has_path_component(path, ".cargo")
            && (has_path_component(path, "registry") || has_path_component(path, "git")))
}

/// Return whether a path contains one named normal component.
fn has_path_component(path: &Path, expected: &str) -> bool {
    path.components()
        .any(|component| matches!(component, Component::Normal(name) if name == expected))
}

/// Read a source file into a string for exact text fallback checks.
fn read_file(path: &Path) -> Option<String> {
    let mut source = String::new();

    // Raw-placeholder migration diagnostics need exact source snippets even for cfg-disabled code.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(source)
}

/// Collect textual aliases for `thiserror::Error` imports in raw source.
fn source_thiserror_derive_names(source: &str) -> BTreeSet<String> {
    // Record the ordinary imported derive name when present.
    let mut aliases = BTreeSet::new();

    if source.contains("use thiserror::Error;") {
        let _ = aliases.insert("Error".to_owned());
    }

    // Parse simple renamed imports one source line at a time.
    for line in source.lines() {
        let Some(alias) = line
            .trim()
            .strip_prefix("use thiserror::Error as ")
            .and_then(|rest| rest.trim_end_matches(';').split_whitespace().next())
        else {
            continue;
        };
        let _ = aliases.insert(alias.to_owned());
    }

    aliases
}

/// Return whether raw source contains a derive attribute for `thiserror::Error`.
fn source_derives_thiserror_error(source: &str, derive_names: &BTreeSet<String>) -> bool {
    attr_ranges(source, "derive")
        .into_iter()
        .any(|(start, end)| {
            source.get(start..end).is_some_and(|attribute| {
                attribute
                    .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
                    .any(|segment| segment == "thiserror::Error" || derive_names.contains(segment))
            })
        })
}

/// Return byte ranges for attributes whose body starts with the target name.
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

        // Retain attributes whose body begins with the target name.
        let body = source
            .get(start + 2..end - 1)
            .map(str::trim_start)
            .unwrap_or_default();
        if body.starts_with(name) {
            ranges.push((start, end));
        }
        search_start = end;
    }

    ranges
}

/// Parse the first string literal out of a raw source `#[error(...)]` attribute.
fn error_format_literal_from_source(attr: &str) -> Option<String> {
    // Locate the attribute argument list and require a leading string literal.
    let chars = error_literal_chars(attr)?;

    // Decode escaped characters while retaining the literal's logical contents.
    let mut literal = String::new();
    let mut escaped = false;
    for ch in chars {
        // Treat a backslash as escaping exactly the next source character.
        if escaped {
            literal.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(literal);
        } else {
            literal.push(ch);
        }
    }

    None
}

/// Return the characters after the opening quote of an error format literal.
fn error_literal_chars(attr: &str) -> Option<std::str::Chars<'_>> {
    let argument_start = attr.find('(')? + 1;
    let mut chars = attr.get(argument_start..)?.trim_start().chars();
    (chars.next() == Some('"')).then_some(chars)
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

/// Return whether an item derives `thiserror::Error` through a qualified path or alias.
fn derives_thiserror_error(attrs: &[Attribute], derive_names: &BTreeSet<Symbol>) -> bool {
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

/// Return whether one derive meta item names `thiserror::Error`.
fn is_thiserror_error_derive(meta: &MetaItem, derive_names: &BTreeSet<Symbol>) -> bool {
    let segments: Vec<_> = meta
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.name)
        .collect();

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

/// Read source text for the span covered by an attribute.
fn attr_snippet(cx: &EarlyContext<'_>, span: Span) -> Option<String> {
    cx.sess().source_map().span_to_snippet(span).ok()
}

/// Build a span for a byte range inside the source file currently being scanned.
fn range_span(start_pos: BytePos, start: usize, end: usize) -> Span {
    let lo = start_pos + byte_pos(start);
    let hi = start_pos + byte_pos(end);

    Span::new(lo, hi, SyntaxContext::root(), None)
}

/// Convert a source offset into rustc's bounded byte-position type.
fn byte_pos(offset: usize) -> BytePos {
    BytePos(u32::try_from(offset).expect("source file offset exceeds rustc BytePos range"))
}

/// Intern an attribute or path segment name for rustc AST comparisons.
fn symbol(name: &str) -> Symbol {
    Symbol::intern(name)
}

/// Emit a machine-applicable rewrite for the raw-field placeholder.
fn emit_span_lint_with_suggestion(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: String,
) {
    // The replacement keeps the same attribute and only changes the documented placeholder form.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_suggestion = diag.span_suggestion(
                span,
                help,
                suggestion.clone(),
                Applicability::MachineApplicable,
            );
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
