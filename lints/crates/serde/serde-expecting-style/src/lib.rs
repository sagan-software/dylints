#![feature(rustc_private)]

//! A lint to check for serde expecting messages that do not match Serde style.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    emit_span_lint_with_help, emit_span_lint_with_suggestion, loaded_rust_sources, parse_items,
    range_span, serde_attr_value,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_EXPECTING_STYLE,
    Warn,
    "`serde(expecting)` message should be a lowercase noun phrase without a period",
    SerdeExpectingStyle
}

impl EarlyLintPass for SerdeExpectingStyle {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, _krate: &Crate) {
        for candidate in loaded_rust_sources(cx) {
            check_source(cx, &candidate);
        }
    }
}

/// Check one source file for `serde(expecting)` values that need style normalization.
fn check_source(cx: &EarlyContext<'_>, candidate: &serde_support::SourceCandidate) {
    // Visit every parsed item's Serde container attributes in source order.
    for item in parse_items(&candidate.source) {
        for attr in item
            .attrs
            .iter()
            .filter(|attr| attr.name(&candidate.source) == Some("serde"))
        {
            // Continue only when the attribute has a parseable `expecting` literal.
            let Some(value) = serde_attr_value(&candidate.source, attr, "expecting") else {
                continue;
            };
            let Some(replacement) = normalized_expectation(&value.value) else {
                continue;
            };
            let span = range_span(candidate.start_pos, attr.start, attr.end);
            let message = "`serde(expecting)` should be a lowercase noun phrase without a period";
            let help = "rewrite the expectation text";
            // The parsed value drops escape backslashes, so re-quoting it would change or
            // break an escaped literal. Those literals get help without a rewrite.
            let has_escape = candidate
                .source
                .get(value.literal_start..value.literal_end)
                .is_none_or(|literal| literal.contains('\\'));
            if has_escape {
                emit_span_lint_with_help(cx, SERDE_EXPECTING_STYLE, span, message, help);
                continue;
            }
            // Rebuild the bounded attribute text while retaining surrounding arguments.
            let Some(attr_source) = attr.source(&candidate.source) else {
                continue;
            };
            let mut attr_replacement = attr_source.to_owned();
            attr_replacement.replace_range(
                value.literal_start - attr.start..value.literal_end - attr.start,
                &format!("\"{replacement}\""),
            );

            // Suggest the complete attribute because the source offsets are exact.
            emit_span_lint_with_suggestion(
                cx,
                SERDE_EXPECTING_STYLE,
                span,
                message,
                help,
                attr_replacement,
                Applicability::MachineApplicable,
            );
        }
    }
}

/// Return a style-normalized expectation string when the original needs rewriting.
fn normalized_expectation(value: &str) -> Option<String> {
    // Remove one terminal period before normalizing the initial ASCII letter.
    let mut normalized = value.to_owned();
    if normalized.ends_with('.') {
        let _ = normalized.pop();
    }
    // Preserve unchanged values by returning no suggestion.
    if let Some(first) = normalized.get_mut(..1) {
        first.make_ascii_lowercase();
    }

    (normalized != value).then_some(normalized)
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
