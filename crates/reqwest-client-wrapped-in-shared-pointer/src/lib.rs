#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Reqwest clients wrapped in redundant shared pointers.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{arc_or_rc_argument, emit_span_lint_with_help, is_async_reqwest_client};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_CLIENT_WRAPPED_IN_SHARED_POINTER,
    Warn,
    "a Reqwest client is wrapped in a redundant Arc or Rc",
    ReqwestClientWrappedInSharedPointer
}

impl<'tcx> LateLintPass<'tcx> for ReqwestClientWrappedInSharedPointer {
    /// Check the argument to resolved shared-pointer constructors.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Report only resolved shared-pointer constructors wrapping an async client.
        let Some(argument) = arc_or_rc_argument(cx, expr) else {
            return;
        };
        if !is_async_reqwest_client(cx, argument) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_CLIENT_WRAPPED_IN_SHARED_POINTER,
            expr.span,
            "Reqwest `Client` already uses `Arc` internally",
            "store and clone the `Client` directly",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
