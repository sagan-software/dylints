#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Tokio sleeps in loops.
//!
//! The lint resolves calls to `tokio::time::sleep` and reports those inside a
//! loop of the same function or closure body, where repeated sleeps
//! accumulate timing drift. It recommends `tokio::time::interval`.
//! The check follows only resolved Tokio calls and keeps nested closure
//! boundaries separate.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, is_in_loop, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_SLEEP_IN_LOOP,
    Warn,
    "Tokio sleep is used for repeated loop timing",
    TokioSleepInLoop
}

impl<'tcx> LateLintPass<'tcx> for TokioSleepInLoop {
    /// Check resolved sleep calls nested in loops.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some((span, _)) = tokio_function_call(cx, expr, "tokio::time::sleep::sleep") else {
            return;
        };
        if is_in_loop(cx, expr) {
            emit(
                cx,
                TOKIO_SLEEP_IN_LOOP,
                span,
                "repeated sleep timing accumulates drift",
                "use `tokio::time::interval` for periodic work",
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
