#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Tokio sleeps in loops.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Tokio sleep calls, reports sleeps inside loops,
//! and recommends an explicit scheduling or timing design for the loop.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
#[cfg(test)]
use tokio as _;
use tokio_support::{is_in_loop, tokio_function_call};

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
        // Resolve the exact sleep function and require lexical loop ancestry.
        let Some((span, _)) = tokio_function_call(cx, expr, "tokio::time::sleep::sleep") else {
            return;
        };
        if !is_in_loop(cx, expr) {
            return;
        }
        cx.emit_span_lint(
            TOKIO_SLEEP_IN_LOOP,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("repeated sleep timing accumulates drift")
                    .help("use `tokio::time::interval` for periodic work");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
