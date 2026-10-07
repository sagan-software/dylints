#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Reqwest retry policies without budgets.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, reqwest_method};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_RETRY_NO_BUDGET,
    Warn,
    "a Reqwest retry policy disables its retry budget",
    ReqwestRetryNoBudget
}

impl<'tcx> LateLintPass<'tcx> for ReqwestRetryNoBudget {
    /// Check resolved calls to Reqwest's explicitly discouraged method.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Report only resolved retry policy builders configured without a budget.
        let Some(method) = reqwest_method(cx, expr) else {
            return;
        };
        if method.name.as_str() != "no_budget" {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_RETRY_NO_BUDGET,
            method.span,
            "this retry policy has an infinite budget and can amplify retry storms",
            "keep the default retry budget or configure a finite `max_extra_load`",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
