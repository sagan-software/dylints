#![feature(rustc_private)]

//! A lint to check for serde untagged enums without expecting messages.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Serde enum attributes, reports untagged enums
//! without an expectation message, and recommends documenting invalid input.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ItemKind, ast_has_serde_attr, ast_serde_attr, emit_span_lint_with_help, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_UNTAGGED_MISSING_EXPECTING,
    Warn,
    "`serde(untagged)` enum should provide an expecting message",
    SerdeUntaggedMissingExpecting
}

impl EarlyLintPass for SerdeUntaggedMissingExpecting {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check untagged deserializable enums for a custom expecting message.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde items before checking untagged enum diagnostics.
    let krate = serde_ast_crate(cx, krate);

    // Report deserializable untagged enums without a custom expectation.
    for item in &krate.items {
        if item.kind != ItemKind::Enum || !item.derives.has_deserialize {
            continue;
        }
        let Some(untagged_attr) = ast_serde_attr(cx, item.attrs, "untagged") else {
            continue;
        };
        if ast_has_serde_attr(cx, item.attrs, "expecting") {
            continue;
        }

        emit_span_lint_with_help(
            cx,
            SERDE_UNTAGGED_MISSING_EXPECTING,
            untagged_attr.span,
            "`serde(untagged)` does not produce informative fallback errors by default",
            "add `expecting = \"...\"` with a domain-specific description of accepted input",
        );
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
