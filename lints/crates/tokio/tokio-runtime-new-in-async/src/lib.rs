#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks runtime construction inside async code.
//!
//! The lint resolves calls to `tokio::runtime::Runtime::new` and reports those
//! whose nearest enclosing body is async, because dropping the new runtime
//! there panics. Reusing the active runtime preserves executor ownership and
//! avoids constructing a nested scheduler inside the future.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, is_in_async_body, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_RUNTIME_NEW_IN_ASYNC,
    Warn,
    "a Tokio runtime is created in async code",
    TokioRuntimeNewInAsync
}

impl<'tcx> LateLintPass<'tcx> for TokioRuntimeNewInAsync {
    /// Check direct runtime construction in the nearest async body.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some((span, _)) =
            tokio_function_call(cx, expr, "tokio::runtime::runtime::Runtime::new")
        else {
            return;
        };
        if is_in_async_body(cx, expr) {
            emit(
                cx,
                TOKIO_RUNTIME_NEW_IN_ASYNC,
                span,
                "dropping this runtime in async code can panic",
                "reuse the active runtime",
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
