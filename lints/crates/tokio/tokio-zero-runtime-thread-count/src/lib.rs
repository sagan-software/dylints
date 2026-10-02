#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero Tokio runtime thread counts.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Tokio runtime builders, reports zero worker
//! counts, and recommends a positive runtime thread count.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tokio as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;
use tokio_support::{is_zero_integer, tokio_method};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_ZERO_RUNTIME_THREAD_COUNT,
    Warn,
    "a Tokio runtime thread count is configured as zero",
    TokioZeroRuntimeThreadCount
}

impl<'tcx> LateLintPass<'tcx> for TokioZeroRuntimeThreadCount {
    /// Check one Tokio runtime builder method call.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the method and restrict analysis to thread-count setters.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        if !matches!(
            method.name.as_str(),
            "worker_threads" | "max_blocking_threads"
        ) {
            return;
        }
        let ExprKind::MethodCall(_, _, [count], _) = expr.kind else {
            return;
        };
        // Report only the literal zero value rejected by Tokio at runtime.
        if !is_zero_integer(count) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            TOKIO_ZERO_RUNTIME_THREAD_COUNT,
            count.span,
            "Tokio runtime thread count must be greater than zero",
            "use a positive count, or remove this configuration to use Tokio's default",
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
