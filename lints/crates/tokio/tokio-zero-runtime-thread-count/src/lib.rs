#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero Tokio runtime thread counts.
//!
//! The lint resolves `worker_threads` and `max_blocking_threads` calls on
//! `tokio::runtime::Builder` and reports a literal zero count, which makes
//! Tokio panic. A positive count gives the runtime an executable worker pool
//! and keeps the configuration explicit.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, is_zero_integer, tokio_method};

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
        // Resolve the builder method before inspecting its argument.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        // Restrict the finding to the two thread-count setters.
        if !matches!(
            method.definition_name.as_str(),
            "tokio::runtime::builder::Builder::worker_threads"
                | "tokio::runtime::builder::Builder::max_blocking_threads"
        ) {
            return;
        }
        if let ExprKind::MethodCall(_, _, [count], _) = expr.kind
            && is_zero_integer(count)
        {
            emit(
                cx,
                TOKIO_ZERO_RUNTIME_THREAD_COUNT,
                count.span,
                "Tokio runtime thread count must be greater than zero",
                "use a positive count, or remove this configuration to use Tokio's default",
                None,
            );
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
