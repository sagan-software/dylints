#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero-capacity Tokio channels.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tokio as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;
use tokio_support::{is_zero_integer, tokio_function_arguments};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_ZERO_CAPACITY_CHANNEL,
    Warn,
    "a Tokio channel is constructed with zero capacity",
    TokioZeroCapacityChannel
}

impl<'tcx> LateLintPass<'tcx> for TokioZeroCapacityChannel {
    /// Check one call to a bounded Tokio channel constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Keep the supported bounded-channel definitions as a closed path set.
        const CHANNEL_PATHS: [&str; 2] = [
            "tokio::sync::broadcast::channel",
            "tokio::sync::mpsc::bounded::channel",
        ];

        // Resolve each constructor and report its sole capacity only when zero.
        for path in CHANNEL_PATHS {
            let Some([capacity]) = tokio_function_arguments(cx, expr, path) else {
                continue;
            };
            if !is_zero_integer(capacity) {
                return;
            }

            emit_span_lint_with_help(
                cx,
                TOKIO_ZERO_CAPACITY_CHANNEL,
                capacity.span,
                "Tokio channel capacity must be greater than zero",
                "use a positive capacity",
            );
            return;
        }
    }
}

/// Emit the diagnostic with Tokio's documented constraint.
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
