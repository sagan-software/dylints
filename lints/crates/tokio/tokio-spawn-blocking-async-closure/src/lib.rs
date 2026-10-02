#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks async closures passed to `spawn_blocking`.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Tokio blocking-task calls, reports async
//! closures passed to them, and recommends a synchronous blocking closure.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
#[cfg(test)]
use tokio as _;
use tokio_support::{is_async_closure, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_SPAWN_BLOCKING_ASYNC_CLOSURE,
    Warn,
    "spawn_blocking receives an async closure",
    TokioSpawnBlockingAsyncClosure
}

impl<'tcx> LateLintPass<'tcx> for TokioSpawnBlockingAsyncClosure {
    /// Check the resolved Tokio function and closure kind.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Require the exact one-closure call before classifying async behavior.
        let Some((span, [closure])) =
            tokio_function_call(cx, expr, "tokio::task::blocking::spawn_blocking")
        else {
            return;
        };
        if !is_async_closure(cx, closure) {
            return;
        }
        cx.emit_span_lint(
            TOKIO_SPAWN_BLOCKING_ASYNC_CLOSURE,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("the future returned by this closure is not polled")
                    .help("use `tokio::spawn` or a synchronous closure");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
