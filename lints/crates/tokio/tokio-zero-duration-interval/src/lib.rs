#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero-duration Tokio intervals.
//!
//! The lint resolves calls to `tokio::time::interval` and `interval_at` and
//! reports a period that is a compile-time zero `Duration`, which makes Tokio
//! panic. A positive period keeps scheduling cooperative and makes the polling
//! cadence explicit for callers.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, is_zero_duration, tokio_function_call};

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
        let period = tokio_function_call(cx, expr, "tokio::time::interval::interval")
            .and_then(|(_, arguments)| arguments.first())
            .or_else(|| {
                tokio_function_call(cx, expr, "tokio::time::interval::interval_at")
                    .and_then(|(_, arguments)| arguments.get(1))
            });
        if let Some(period) = period
            && is_zero_duration(cx, period)
        {
            emit(
                cx,
                TOKIO_ZERO_DURATION_INTERVAL,
                period.span,
                "Tokio interval period must be greater than zero",
                "use a positive duration",
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
