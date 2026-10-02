#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks positional formatting of named thiserror fields.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::Attribute;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::Symbol;
#[cfg(test)]
use thiserror as _;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_NAMED_FIELD_POSITIONAL_FORMAT,
    Warn,
    "a named thiserror field is formatted positionally",
    ThiserrorNamedFieldPositionalFormat
}

impl EarlyLintPass for ThiserrorNamedFieldPositionalFormat {
    /// Check the source shape of one `#[error(...)]` attribute.
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        // Restrict source inspection to thiserror display attributes.
        if !attr.has_name(Symbol::intern("error")) {
            return;
        }
        let Ok(snippet) = cx.sess().source_map().span_to_snippet(attr.span) else {
            return;
        };
        if !snippet.contains("{}\",") {
            return;
        }

        let replacement = positional_field_replacement(&snippet);

        // Point at the complete attribute whose format string needs a named capture.
        cx.emit_span_lint(
            THISERROR_NAMED_FIELD_POSITIONAL_FORMAT,
            attr.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic =
                    diagnostic.primary_message("this named field is formatted positionally");
                if let Some(replacement) = replacement {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        attr.span,
                        "capture the field directly as `{field}`",
                        replacement,
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help =
                        diagnostic.help("capture the field directly as `{field}`");
                }
            }),
        );
    }
}

/// Build a safe whole-attribute rewrite for the simple positional-field form.
fn positional_field_replacement(snippet: &str) -> Option<String> {
    let snippet = snippet.trim();
    // Parse the complete source shape before constructing any replacement text.
    let parts = parse_attribute_parts(snippet)?;

    // Delegate source slicing so this public fixer entry point has one fallible boundary.
    let replacement_literal = replacement_literal(snippet, parts)?;
    assemble_replacement(snippet, parts, &replacement_literal)
}

/// Build the rewritten quoted literal for one parsed attribute.
fn replacement_literal(snippet: &str, parts: AttributeParts) -> Option<String> {
    let format_literal = snippet.get(parts.format_start..parts.format_end)?;
    let field = snippet.get(parts.field_start..parts.field_end)?;
    rewritten_format_literal(format_literal, parts.placeholder, field)
}

/// Assemble the rewritten attribute while retaining all unrelated source text.
fn assemble_replacement(
    snippet: &str,
    parts: AttributeParts,
    replacement_literal: &str,
) -> Option<String> {
    // Preserve every source byte outside the one placeholder and field name.
    let prefix = snippet.get(..parts.format_start)?;
    let suffix = snippet.get(parts.field_end..)?;
    let mut replacement =
        String::with_capacity(snippet.len() + parts.field_end - parts.field_start);
    // Append the rewritten literal between the untouched source boundaries.
    replacement.push_str(prefix);
    replacement.push_str(replacement_literal);
    replacement.push_str(suffix);
    Some(replacement)
}

/// Source offsets needed to rewrite one simple thiserror attribute.
#[derive(Clone, Copy, Debug)]
struct AttributeParts {
    /// Start of the quoted format literal.
    format_start: usize,
    /// End of the quoted format literal.
    format_end: usize,
    /// Start of the named field argument.
    field_start: usize,
    /// End of the named field argument.
    field_end: usize,
    /// Byte offset of the only empty placeholder inside the literal.
    placeholder: usize,
}

/// Parse all source offsets required by the machine-applicable rewrite.
fn parse_attribute_parts(snippet: &str) -> Option<AttributeParts> {
    let (arguments_start, arguments_end) = attribute_bounds(snippet)?;
    let (format_start, format_end, field_start, field_end) =
        format_and_field_bounds(snippet, arguments_start, arguments_end)?;
    let placeholder = placeholder_in_literal(snippet, format_start, format_end)?;

    Some(AttributeParts {
        format_start,
        format_end,
        field_start,
        field_end,
        placeholder,
    })
}

/// Return the argument-list bounds for one complete error attribute.
fn attribute_bounds(snippet: &str) -> Option<(usize, usize)> {
    // Require the exact attribute wrapper before inspecting its arguments.
    if !snippet.starts_with("#[error(") || !snippet.ends_with(")]") {
        return None;
    }
    Some(("#[error(".len(), snippet.len().checked_sub(2)?))
}

/// Parse the format and field offsets while preserving their source ordering.
fn format_and_field_bounds(
    snippet: &str,
    arguments_start: usize,
    arguments_end: usize,
) -> Option<(usize, usize, usize, usize)> {
    let (format_start, format_end) = format_bounds(snippet, arguments_start, arguments_end)?;
    let comma = format_argument_comma(snippet, format_end, arguments_end)?;
    let (field_start, field_end) = field_bounds(snippet, comma, arguments_end)?;
    Some((format_start, format_end, field_start, field_end))
}

/// Return the quoted format literal bounds.
fn format_bounds(
    snippet: &str,
    arguments_start: usize,
    arguments_end: usize,
) -> Option<(usize, usize)> {
    // Ignore whitespace before the first format literal.
    let arguments = snippet.get(arguments_start..arguments_end)?;
    let format_start = arguments_start + (arguments.len() - arguments.trim_start().len());
    if !snippet.get(format_start..arguments_end)?.starts_with('"') {
        return None;
    }
    Some((format_start, quoted_literal_end(snippet, format_start)?))
}

/// Return the comma separating the format literal and named field.
fn format_argument_comma(snippet: &str, format_end: usize, arguments_end: usize) -> Option<usize> {
    // Require only whitespace between the literal and its separating comma.
    let separator = snippet.get(format_end..arguments_end)?;
    let comma = separator.find(',')? + format_end;
    snippet
        .get(format_end..comma)
        .filter(|source| source.trim().is_empty())
        .map(|_| comma)
}

/// Return the trimmed named field argument bounds.
fn field_bounds(snippet: &str, comma: usize, arguments_end: usize) -> Option<(usize, usize)> {
    // Restrict the replacement to one simple identifier argument.
    let field_start = comma + 1;
    let field_source = snippet.get(field_start..arguments_end)?;

    // Trim only surrounding whitespace so the machine-applicable span remains exact.
    let field = field_source.trim();
    if !is_simple_field_name(field) {
        return None;
    }
    let field_start = field_start + (field_source.len() - field.len());
    Some((field_start, field_start + field.len()))
}

/// Return the only empty placeholder offset inside a quoted format literal.
fn placeholder_in_literal(snippet: &str, format_start: usize, format_end: usize) -> Option<usize> {
    // Exclude the surrounding quotes before scanning format syntax.
    let format_literal = snippet.get(format_start..format_end)?;

    // Validate the interior as a single unescaped empty formatting placeholder.
    let content = format_literal.get(1..format_literal.len().checked_sub(1)?)?;
    one_empty_placeholder(content)
}

/// Rewrite one empty placeholder while preserving the rest of the format literal.
fn rewritten_format_literal(
    format_literal: &str,
    placeholder: usize,
    field: &str,
) -> Option<String> {
    // Preserve the literal prefix and suffix around the placeholder.
    let prefix = format_literal.get(..placeholder + 1)?;
    // Include the remainder after the original empty placeholder unchanged.
    let suffix = format_literal.get(placeholder + 3..)?;
    Some(format!("{prefix}{{{field}}}{suffix}"))
}

/// Find the closing quote of an ordinary Rust string literal.
fn quoted_literal_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;

    // Walk byte offsets because rustc spans and source slices use byte positions.
    while index < bytes.len() {
        let byte = bytes.get(index)?;
        match *byte {
            // Skip the escaped byte together with its backslash.
            b'\\' => index = index.saturating_add(2),
            // Include the closing quote in the returned range.
            b'"' => return Some(index.saturating_add(1)),
            _ => index += 1,
        }
    }
    None
}

/// Return the only unescaped `{}` placeholder in a format string.
fn one_empty_placeholder(content: &str) -> Option<usize> {
    let bytes = content.as_bytes();
    let mut index = 0;
    let mut placeholder = None;

    // Advance over ordinary text and validate each brace pair independently.
    while index < bytes.len() {
        index = placeholder_step(bytes, index, &mut placeholder)?;
    }
    // Return the unique placeholder only after the complete format was checked.
    placeholder
}

/// Validate one brace pair and return the next content offset.
fn placeholder_step(bytes: &[u8], index: usize, placeholder: &mut Option<usize>) -> Option<usize> {
    // Ordinary text and unmatched closing braces cannot form a placeholder.
    match bytes.get(index).copied()? {
        byte if byte != b'{' => (byte != b'}').then_some(index + 1),
        // Accept escaped opening braces without recording a replacement.
        b'{' => match bytes.get(index + 1).copied()? {
            b'{' => Some(index + 2),
            // Permit exactly one empty placeholder in the complete format string.
            b'}' => placeholder.replace(index).is_none().then_some(index + 2),
            _ => None,
        },
        _ => None,
    }
}

/// Return whether a source argument is one simple ASCII field name.
fn is_simple_field_name(field: &str) -> bool {
    let mut characters = field.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    use super::positional_field_replacement;

    #[test]
    fn rewrites_the_simple_attribute_shape() {
        assert_eq!(
            positional_field_replacement("#[error(\"failed: {}\", source)]"),
            Some("#[error(\"failed: {source}\")]".to_owned())
        );
    }

    #[test]
    #[expect(
        clippy::literal_string_with_formatting_args,
        reason = "the literals are attribute sources under test, not format strings"
    )]
    fn rejects_ambiguous_format_shapes() {
        for source in [
            "#[error(r#\"failed: {}\"#, source)]",
            "#[error(\"failed: {} {}\", source)]",
            "#[error(\"failed: {{}}\", source)]",
            "#[error(\"failed: {:?}\", source)]",
            "#[error(\"failed: {}\", source.value)]",
        ] {
            assert_eq!(positional_field_replacement(source), None, "{source}");
        }
    }
}
