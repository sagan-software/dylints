#![feature(rustc_private)]

//! A lint to check for nonportable Serde `deserialize_any` calls.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

use serde_support::serde_method_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_DESERIALIZE_ANY_PORTABILITY,
    Warn,
    "serde deserialize_any limits deserialization to self-describing formats",
    SerdeDeserializeAnyPortability
}

impl<'tcx> LateLintPass<'tcx> for SerdeDeserializeAnyPortability {
    /// Check semantically resolved Serde method calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(method) = serde_method_call(cx, expr, "de::Deserializer", "deserialize_any")
        else {
            return;
        };

        emit_span_lint_with_help(
            cx,
            SERDE_DESERIALIZE_ANY_PORTABILITY,
            method.method_span,
            "`deserialize_any` restricts this code to self-describing formats",
            "use a typed `deserialize_*` method unless dynamic input is intentional",
        );
    }
}

/// Emit a late lint with portability guidance.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic.primary_message(message).help(help);
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
