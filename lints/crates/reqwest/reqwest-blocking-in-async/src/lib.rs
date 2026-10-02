#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Reqwest blocking APIs in async bodies.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{
    emit_span_lint_with_help, is_in_async_body, reqwest_function, reqwest_method,
};
use rustc_hir::{Expr, ExprKind, Node};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_BLOCKING_IN_ASYNC,
    Warn,
    "a Reqwest blocking API is called in an async body",
    ReqwestBlockingInAsync
}

impl<'tcx> LateLintPass<'tcx> for ReqwestBlockingInAsync {
    /// Check one resolved Reqwest call against its nearest async boundary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Reject synchronous bodies before resolving Reqwest call paths.
        if !is_in_async_body(cx, expr) {
            return;
        }

        // Normalize method and free-function calls to one span and definition pair.
        let call = reqwest_method(cx, expr)
            .map(|method| (method.span, method.definition))
            .or_else(|| {
                reqwest_function(cx, expr).map(|function| (function.span, function.definition))
            });
        let Some((span, path)) = call else {
            return;
        };
        if !path.contains("::blocking::") {
            return;
        }
        if is_receiver_of_blocking_call(cx, expr) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_BLOCKING_IN_ASYNC,
            span,
            "Reqwest's blocking API can panic and block the executor in an async body",
            "use the asynchronous Reqwest API, or move the whole operation into `spawn_blocking`",
        );
    }
}

/// Suppress inner links in one fluent blocking call chain.
fn is_receiver_of_blocking_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Suppress nested receiver nodes when the parent method is already diagnostic-worthy.
    let Some((_, Node::Expr(parent))) = cx.tcx.hir_parent_iter(expr.hir_id).next() else {
        return false;
    };
    let ExprKind::MethodCall(_, receiver, _, _) = parent.kind else {
        return false;
    };

    receiver.hir_id == expr.hir_id
        && reqwest_method(cx, parent)
            .is_some_and(|method| method.definition.contains("::blocking::"))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
