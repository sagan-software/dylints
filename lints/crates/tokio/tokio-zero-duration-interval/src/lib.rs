#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero-duration Tokio intervals.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tokio as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;
use tokio_support::{is_zero_duration, tokio_function_arguments};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_ZERO_DURATION_INTERVAL,
    Warn,
    "a Tokio interval is constructed with a zero period",
    TokioZeroDurationInterval
}

impl<'tcx> LateLintPass<'tcx> for TokioZeroDurationInterval {
    /// Check one call to a Tokio interval constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Normalize both interval constructors to their period argument.
        let period = tokio_function_arguments(cx, expr, "tokio::time::interval::interval")
            .and_then(|arguments| arguments.first())
            .or_else(|| {
                tokio_function_arguments(cx, expr, "tokio::time::interval::interval_at")
                    .and_then(|arguments| arguments.get(1))
            });
        let Some(period) = period else {
            return;
        };
        // Report only periods semantically equal to zero duration.
        if !is_zero_duration(cx, period) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            TOKIO_ZERO_DURATION_INTERVAL,
            period.span,
            "Tokio interval period must be greater than zero",
            "use a positive duration",
        );
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
