#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks runtime construction inside async code.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
#[cfg(test)]
use tokio as _;
use tokio_support::{is_in_async_body, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_RUNTIME_NEW_IN_ASYNC,
    Warn,
    "a Tokio runtime is constructed in async code",
    TokioRuntimeNewInAsync
}

impl<'tcx> LateLintPass<'tcx> for TokioRuntimeNewInAsync {
    /// Check direct runtime construction in the nearest async body.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Require exact runtime construction within the nearest async boundary.
        let Some((span, _)) =
            tokio_function_call(cx, expr, "tokio::runtime::runtime::Runtime::new")
        else {
            return;
        };
        if !is_in_async_body(cx, expr) {
            return;
        }
        cx.emit_span_lint(
            TOKIO_RUNTIME_NEW_IN_ASYNC,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("dropping this runtime in async code can panic")
                    .help("reuse the active runtime");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
