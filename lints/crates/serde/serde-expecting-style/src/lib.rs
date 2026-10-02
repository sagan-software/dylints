#![feature(rustc_private)]

//! A lint to check for serde expecting messages that do not match Serde style.
//!
//! This Dylint library reads `#[serde(expecting = "...")]` from the HIR
//! attributes of structs and enums, and rewrites the literal when only its
//! first letter or its final period needs to change.

extern crate rustc_ast;
extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_ast::{LitKind, StrStyle};
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};

use serde_support::{Help, attr_entries, emit_lint, entry_literal, namespace_attrs};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_EXPECTING_STYLE,
    Warn,
    "`serde(expecting)` message should be a lowercase noun phrase without a period",
    SerdeExpectingStyle
}

impl<'tcx> LateLintPass<'tcx> for SerdeExpectingStyle {
    /// Check the container attributes of one struct or enum.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // `expecting` is a container attribute of structs and enums.
        if !matches!(item.kind, ItemKind::Struct(..) | ItemKind::Enum(..)) {
            return;
        }
        let attrs = cx.tcx.hir_attrs(item.hir_id());

        // Visit every `expecting = "..."` entry with a string literal.
        for attr in namespace_attrs(attrs, "serde") {
            for entry in attr_entries(attr, "expecting") {
                let Some(literal) = entry_literal(&entry) else {
                    continue;
                };
                let LitKind::Str(value, style) = literal.kind else {
                    continue;
                };
                let Some(normalized) = normalized_expectation(value.as_str()) else {
                    continue;
                };

                // Rewrite the source literal only when each change maps to one plain character.
                let replacement = cx
                    .sess()
                    .source_map()
                    .span_to_snippet(literal.span)
                    .ok()
                    .filter(|_| !literal.span.from_expansion())
                    .and_then(|source| {
                        rewrite_literal(&source, style, value.as_str(), &normalized)
                    });
                // Attach the rewrite when it exists, and plain help otherwise.
                let help = "rewrite the expectation text";
                let help = replacement.map_or_else(
                    || Help::text(help),
                    |replacement| Help::Rewrite {
                        text: help.to_owned(),
                        parts: vec![(literal.span, replacement)],
                    },
                );
                emit_lint(
                    cx,
                    SERDE_EXPECTING_STYLE,
                    item.hir_id(),
                    literal.span,
                    "`serde(expecting)` should be a lowercase noun phrase without a period",
                    help,
                );
            }
        }
    }
}

/// Return the style-normalized expectation, or `None` when the value already conforms.
///
/// The first letter is lowercased only when it starts an ordinary word, so an
/// acronym such as `UUID` or `I/O` keeps its case.
fn normalized_expectation(value: &str) -> Option<String> {
    // Remove one terminal period.
    let mut normalized = value.strip_suffix('.').unwrap_or(value).to_owned();
    // Lowercase a leading capital that a lowercase letter, a space, or the end follows.
    let mut characters = normalized.chars();
    let first = characters.next();
    let second = characters.next();
    let is_capital = first.is_some_and(|first| first.is_ascii_uppercase());
    let is_word_start =
        second.is_none_or(|second| second.is_ascii_lowercase() || second.is_whitespace());
    if is_capital
        && is_word_start
        && let Some(first) = normalized.get_mut(..1)
    {
        first.make_ascii_lowercase();
    }

    (normalized != value).then_some(normalized)
}

/// Rewrite the literal's source text, or return `None` when an escape blocks it.
///
/// The edit works on the source spelling, so escapes elsewhere in the literal
/// stay as written. It applies only when the changed first character and the
/// removed period appear unescaped in the source.
fn rewrite_literal(source: &str, style: StrStyle, value: &str, normalized: &str) -> Option<String> {
    // Split the literal into its delimiters and source contents.
    let (open, close) = match style {
        StrStyle::Cooked => (1, 1),
        StrStyle::Raw(hashes) => (usize::from(hashes) + 2, usize::from(hashes) + 1),
    };
    let contents_end = source.len().checked_sub(close)?;
    let contents = source.get(open..contents_end)?;
    let rewritten = rewrite_contents(contents, value, normalized)?;

    // Reattach the original delimiters around the edited contents.
    Some(format!(
        "{}{rewritten}{}",
        source.get(..open)?,
        source.get(contents_end..)?
    ))
}

/// Apply the period and capital edits to the literal's source contents.
fn rewrite_contents(contents: &str, value: &str, normalized: &str) -> Option<String> {
    // Drop the period only when the source spells it as the final character.
    let is_period_removed = value.ends_with('.') && !normalized.ends_with('.');
    let kept = if is_period_removed {
        contents.strip_suffix('.')?
    } else {
        contents
    };
    let mut rewritten = kept.to_owned();

    // An escaped first character cannot be lowercased by editing one byte.

    let is_first_lowered = value.get(..1) != normalized.get(..1);
    let is_first_plain = rewritten.get(..1) == value.get(..1);
    if is_first_lowered && !is_first_plain {
        return None;
    }
    // Lowercase the first character, which the source spells directly.
    if is_first_lowered && let Some(first) = rewritten.get_mut(..1) {
        first.make_ascii_lowercase();
    }
    Some(rewritten)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    use rustc_ast::StrStyle;

    use super::{normalized_expectation, rewrite_literal};

    /// Keep acronyms and conforming values unchanged.
    #[test]
    fn normalization_keeps_acronyms() {
        // Each input pairs with its expected normalization.
        let cases = [
            ("UUID string", None),
            ("I/O path.", Some("I/O path")),
            ("X", Some("x")),
            ("a user id", None),
            ("", None),
        ];
        let actual: Vec<_> = cases
            .iter()
            .map(|(input, _)| normalized_expectation(input))
            .collect();
        let expected: Vec<_> = cases
            .iter()
            .map(|(_, output)| output.map(str::to_owned))
            .collect();
        assert_eq!(actual, expected);
    }

    /// Rewrite plain, escaped, and raw literals only where the source allows it.
    #[test]
    fn literal_rewrites() {
        // Each case is the literal source, its style, its value, and the expected rewrite.
        let cases = [
            (
                r#""A \"quoted\" id.""#,
                StrStyle::Cooked,
                r#"A "quoted" id."#,
                Some(r#""a \"quoted\" id""#),
            ),
            (
                r##"r#"A "raw" id."#"##,
                StrStyle::Raw(1),
                r#"A "raw" id."#,
                Some(r##"r#"a "raw" id"#"##),
            ),
            (r#""\x41 id""#, StrStyle::Cooked, "A id", None),
            (r#""an id\x2E""#, StrStyle::Cooked, "an id.", None),
        ];
        let actual: Vec<_> = cases
            .iter()
            .map(|&(source, style, value, _)| {
                let normalized = normalized_expectation(value).unwrap_or_default();
                rewrite_literal(source, style, value, &normalized)
            })
            .collect();
        let expected: Vec<_> = cases
            .iter()
            .map(|(.., output)| output.map(str::to_owned))
            .collect();
        assert_eq!(actual, expected);
    }
}
