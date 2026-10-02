#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `Handle::block_on` inside async code.
//!
//! The lint resolves the method to `tokio::runtime::Handle::block_on` and
//! reports calls whose nearest enclosing body is async, where Tokio panics. It
//! suggests awaiting the future instead.
//! The check preserves the resolved method span and uses the shared await suggestion.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{await_suggestion, emit, is_in_async_body, tokio_method};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_HANDLE_BLOCK_ON_IN_ASYNC,
    Warn,
    "Tokio Handle::block_on is called in async code",
    TokioHandleBlockOnInAsync
}

impl<'tcx> LateLintPass<'tcx> for TokioHandleBlockOnInAsync {
    /// Check a resolved handle method in its nearest closure boundary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the method before checking its enclosing async body.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        if method.definition_name != "tokio::runtime::handle::Handle::block_on"
            || !is_in_async_body(cx, expr)
        {
            return;
        }
        emit(
            cx,
            TOKIO_HANDLE_BLOCK_ON_IN_ASYNC,
            method.span,
            "`Handle::block_on` can panic inside a Tokio runtime",
            "await the future directly",
            await_suggestion(cx, expr),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
