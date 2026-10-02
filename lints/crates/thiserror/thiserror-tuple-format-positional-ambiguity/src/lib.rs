#![feature(rustc_private)]

//! A lint to check for ambiguous thiserror tuple positional formatting.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use std::{
    collections::BTreeSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use rustc_ast::{
    Crate,
    ast::{self, ModKind, UseTree, UseTreeKind},
};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::{BytePos, SourceFile, Span, SyntaxContext, symbol::Symbol};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_TUPLE_FORMAT_POSITIONAL_AMBIGUITY,
    Warn,
    "`thiserror` tuple format mixes tuple fields and positional arguments",
    ThiserrorTupleFormatPositionalAmbiguity
}

impl EarlyLintPass for ThiserrorTupleFormatPositionalAmbiguity {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        let derive_names = thiserror_derive_names(krate);

        for item in &krate.items {
            check_item(cx, item, &derive_names);
        }
    }
}

/// Check tuple structs and tuple enum variants that belong to a thiserror error type.
fn check_item(cx: &EarlyContext<'_>, item: &ast::Item, derive_names: &BTreeSet<Symbol>) {
    // Dispatch tuple structs to a focused source-attribute check.
    if let ast::ItemKind::Struct(_, _, data) = &item.kind {
        check_struct_item(cx, item, data.fields(), derive_names);
        return;
    }

    // Dispatch enums to a variant-level source-attribute check.
    if let ast::ItemKind::Enum(_, _, enum_def) = &item.kind {
        check_enum_item(cx, item, enum_def, derive_names);
        return;
    }
    // Preserve module traversal after handling both aggregate item forms.
    if let ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) = &item.kind {
        // Recurse through loaded inline modules to preserve crate-wide coverage.
        for item in items {
            check_item(cx, item, derive_names);
        }
    }
}

/// Check one tuple struct whose source derives thiserror.
fn check_struct_item(
    cx: &EarlyContext<'_>,
    item: &ast::Item,
    fields: &[ast::FieldDef],
    derive_names: &BTreeSet<Symbol>,
) {
    // Ignore named and empty structs because positional formatting cannot target them.
    if !is_tuple_fields(fields) {
        return;
    }

    // Require the adjacent source derive before reading error attributes.
    let item_attrs = source_attrs_before(cx, item.span);
    if source_derives_thiserror_error(&item_attrs, derive_names) {
        check_error_attrs(cx, item_attrs);
    }
}

/// Check tuple variants of an enum whose source derives thiserror.
fn check_enum_item(
    cx: &EarlyContext<'_>,
    item: &ast::Item,
    enum_def: &ast::EnumDef,
    derive_names: &BTreeSet<Symbol>,
) {
    // Establish the enum-level derive before checking its tuple variants.
    let item_attrs = source_attrs_before(cx, item.span);
    if !source_derives_thiserror_error(&item_attrs, derive_names) {
        return;
    }

    // Check only variants whose fields can be addressed positionally.
    enum_def
        .variants
        .iter()
        .filter(|variant| is_tuple_fields(variant.data.fields()))
        .for_each(|variant| check_error_attrs(cx, source_attrs_before(cx, variant.span)));
}

/// Check source attributes for ambiguous tuple-field and argument formatting.
fn check_error_attrs(cx: &EarlyContext<'_>, attrs: Vec<SourceAttr>) {
    // Parse only source attributes named `error`.
    for attr in attrs.into_iter().filter(|attr| attr.has_name("error")) {
        let Some(format) = error_format_literal_from_source(&attr.source) else {
            continue;
        };
        if !has_numeric_capture(&format) || !has_extra_positional_arg(&attr.source) {
            continue;
        }

        // Report only when numeric tuple capture and an unnamed argument coexist.
        emit_span_lint_with_help(
            cx,
            THISERROR_TUPLE_FORMAT_POSITIONAL_AMBIGUITY,
            attr.span,
            "`thiserror` tuple format mixes numeric fields and positional arguments",
            "give the extra format argument a name, such as `value = value()`",
        );
    }
}

/// Return whether all fields are positional tuple fields.
fn is_tuple_fields(fields: &[ast::FieldDef]) -> bool {
    !fields.is_empty() && fields.iter().all(|field| field.ident.is_none())
}

/// Source form of an attribute plus the span for diagnostics.
#[derive(Clone, Debug)]
struct SourceAttr {
    /// Attribute text exactly as it appears in the source file.
    source: String,
    /// Span covering the full attribute.
    span: Span,
}

impl SourceAttr {
    /// Return whether the attribute body starts with the target name.
    fn has_name(&self, name: &str) -> bool {
        let Some(body) = self
            .source
            .trim_start()
            .strip_prefix("#[")
            .and_then(|source| source.trim_end().strip_suffix(']'))
        else {
            return false;
        };

        body.trim_start().starts_with(name)
    }
}

/// Read attributes immediately preceding an item from the original source file.
fn source_attrs_before(cx: &EarlyContext<'_>, span: Span) -> Vec<SourceAttr> {
    // Resolve the rustc source file to readable local source text.
    let source_file = cx.sess().source_map().lookup_source_file(span.lo());
    let Some(path) = source_file_path(&source_file) else {
        return Vec::new();
    };
    let Some(source) = read_file(&path) else {
        return Vec::new();
    };
    let Some(item_start) = source_offset(&source_file, span.lo()) else {
        return Vec::new();
    };

    // Preserve each adjacent attribute's exact text and diagnostic span.
    attr_ranges_before(&source, item_start)
        .into_iter()
        .filter_map(|(start, end)| {
            source.get(start..end).map(|attr_source| SourceAttr {
                source: attr_source.to_owned(),
                span: range_span(source_file.start_pos, start, end),
            })
        })
        .collect()
}

/// Convert a rustc source-file name into a local filesystem path.
fn source_file_path(source_file: &SourceFile) -> Option<PathBuf> {
    source_file.name.clone().into_local_path()
}

/// Read a source file into a string for exact attribute parsing.
fn read_file(path: &Path) -> Option<String> {
    let mut source = String::new();

    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(source)
}

/// Convert a rustc byte position into an offset inside one source file.
fn source_offset(source_file: &SourceFile, pos: BytePos) -> Option<usize> {
    pos.0
        .checked_sub(source_file.start_pos.0)
        .and_then(|offset| usize::try_from(offset).ok())
}

/// Return byte ranges for adjacent attributes immediately before an item.
fn attr_ranges_before(source: &str, item_start: usize) -> Vec<(usize, usize)> {
    // Start immediately before the item after ignoring separating whitespace.
    let mut ranges = Vec::new();
    let mut cursor = skip_whitespace_backward(source, item_start);

    // Walk backward over adjacent attributes immediately above the item.
    while cursor > 0
        && source
            .as_bytes()
            .get(cursor.saturating_sub(1))
            .is_some_and(|byte| *byte == b']')
    {
        let end = cursor;
        let Some(start) = source.get(..end).and_then(|prefix| prefix.rfind("#[")) else {
            break;
        };

        // Continue from the start of the attribute to collect its predecessors.
        ranges.push((start, end));
        cursor = skip_whitespace_backward(source, start);
    }

    ranges.reverse();
    ranges
}

/// Move a byte cursor backward over ASCII whitespace.
fn skip_whitespace_backward(source: &str, mut cursor: usize) -> usize {
    // Source offsets come from rustc byte positions, so byte-level whitespace is sufficient here.
    while cursor > 0
        && source
            .as_bytes()
            .get(cursor.saturating_sub(1))
            .is_some_and(u8::is_ascii_whitespace)
    {
        cursor -= 1;
    }

    cursor
}

/// Return whether source attributes include a `thiserror::Error` derive.
fn source_derives_thiserror_error(attrs: &[SourceAttr], derive_names: &BTreeSet<Symbol>) -> bool {
    attrs.iter().any(|attr| {
        if !attr.has_name("derive") {
            return false;
        }

        attr.source
            .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_' || ch == ':'))
            .any(|segment| segment == "thiserror::Error" || derive_names.contains(&symbol(segment)))
    })
}

/// Return whether a format literal captures a tuple field by numeric index.
fn has_numeric_capture(format: &str) -> bool {
    // Track one character of lookahead to distinguish escaped opening braces.
    let mut chars = format.chars().peekable();

    // Accept the first unescaped opening brace followed by an ASCII digit.
    while let Some(ch) = chars.next() {
        match ch {
            '{' if chars.peek() == Some(&'{') => {
                let _ = chars.next();
            }
            '{' if chars.peek().is_some_and(char::is_ascii_digit) => return true,
            _ => {}
        }
    }

    false
}

/// Return whether an error attribute supplies an unnamed format argument.
fn has_extra_positional_arg(attr: &str) -> bool {
    // Begin after the parsed format literal and optional separating comma.
    let Some((_, after_literal)) = split_error_literal(attr) else {
        return false;
    };

    let mut rest = after_literal.trim_start();
    if let Some(after_comma) = rest.strip_prefix(',') {
        rest = after_comma.trim_start();
    }
    let Some(first_arg) = first_top_level_arg(rest) else {
        return false;
    };

    // An argument is positional when its top level has no assignment.
    !top_level_arg_is_named(first_arg)
}

/// Return the first top-level argument after the error format literal.
fn first_top_level_arg(args: &str) -> Option<&str> {
    // Track nesting and string escapes while scanning the raw argument source.
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;

    for (index, ch) in args.char_indices() {
        // Ignore delimiters inside quoted strings.
        if is_quoted_character_consumed(ch, &mut in_string, &mut escaped) {
            continue;
        }

        match ch {
            // Update delimiter depth outside strings.
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    return args.get(..index).and_then(trimmed_nonempty);
                }
                depth -= 1;
            }
            // A top-level comma terminates the first argument.
            ',' if depth == 0 => return args.get(..index).and_then(trimmed_nonempty),
            _ => {}
        }
    }

    trimmed_nonempty(args)
}

/// Consume one character while scanning a quoted source fragment.
const fn is_quoted_character_consumed(
    character: char,
    in_string: &mut bool,
    escaped: &mut bool,
) -> bool {
    if !*in_string {
        return false;
    }

    match (*escaped, character) {
        (true, _) => *escaped = false,
        (false, '\\') => *escaped = true,
        (false, '"') => *in_string = false,
        (false, _) => {}
    }
    true
}

/// Trim a string and discard empty results.
fn trimmed_nonempty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Return whether a top-level format argument uses `name = value` syntax.
fn top_level_arg_is_named(arg: &str) -> bool {
    // Track nesting and string escapes while scanning one raw argument.
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = arg.char_indices().peekable();

    while let Some((_, ch)) = chars.next() {
        // Ignore assignment-like characters inside quoted strings.
        if is_quoted_character_consumed(ch, &mut in_string, &mut escaped) {
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            // A single top-level equals sign marks a named format argument.
            '=' if depth == 0 => return chars.peek().is_none_or(|(_, next)| *next != '='),
            _ => {}
        }
    }

    false
}

/// Parse the format literal from a raw source `#[error(...)]` attribute.
fn error_format_literal_from_source(attr: &str) -> Option<String> {
    let (literal, _) = split_error_literal(attr)?;
    Some(literal)
}

/// Split an error attribute into its format literal and trailing arguments.
fn split_error_literal(attr: &str) -> Option<(String, &str)> {
    // Locate the attribute argument list and require a leading string literal.
    let start = attr.find('(')? + 1;
    let after_parenthesis = attr.get(start..)?;
    let after_open = after_parenthesis.trim_start();
    let skipped = after_parenthesis.len() - after_open.len();
    let (literal, consumed) = decode_quoted_literal(after_open)?;

    // Return the decoded literal and untouched trailing argument source.
    let rest_start = start + skipped + consumed;
    Some((literal, attr.get(rest_start..)?))
}

/// Decode one quoted source literal and return its consumed byte length.
fn decode_quoted_literal(source: &str) -> Option<(String, usize)> {
    // Require the first non-whitespace source character to open a string.
    let mut chars = source.char_indices();
    if chars.next()?.1 != '"' {
        return None;
    }

    // Decode escaped characters while retaining the literal's logical contents.
    let mut literal = String::new();
    let mut escaped = false;

    // Keep the returned byte offset aligned with the original source slice.
    for (index, character) in chars {
        if escaped {
            literal.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some((literal, index + character.len_utf8()));
        } else {
            literal.push(character);
        }
    }

    None
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

/// Intern an attribute or path segment name for rustc AST comparisons.
fn symbol(name: &str) -> Symbol {
    Symbol::intern(name)
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

/// Emit the ambiguity diagnostic with help text because the fix needs a chosen name.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // The fix needs a human-chosen argument name, so keep this diagnostic help-only.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_help = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "main");
}
