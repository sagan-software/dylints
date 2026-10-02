#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for leaking `SQLx` pool connections.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use sqlx as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use sqlx_support::sqlx_method_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SQLX_POOL_CONNECTION_LEAK,
    Warn,
    "SQLx pool connection is permanently checked out",
    SqlxPoolConnectionLeak
}

impl<'tcx> LateLintPass<'tcx> for SqlxPoolConnectionLeak {
    /// Check semantically resolved calls to `PoolConnection::leak`.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion()
            || sqlx_method_call(cx, expr, "PoolConnection", &["leak"]).is_none()
        {
            return;
        }

        cx.emit_span_lint(
            SQLX_POOL_CONNECTION_LEAK,
            expr.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this permanently reduces the SQLx pool's capacity")
                    .help("use `PoolConnection::detach` when the pool should open a replacement");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
