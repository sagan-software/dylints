#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for explicit `SQLx` dynamic SQL safety assertions.
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
use sqlx_support::assert_sql_safe_argument;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SQLX_ASSERT_SQL_SAFE,
    Warn,
    "SQLx dynamic SQL is asserted to be injection-safe",
    SqlxAssertSqlSafe
}

impl<'tcx> LateLintPass<'tcx> for SqlxAssertSqlSafe {
    /// Check direct, user-written construction of `SQLx`'s safety assertion.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() || assert_sql_safe_argument(cx, expr).is_none() {
            return;
        }

        cx.emit_span_lint(
            SQLX_ASSERT_SQL_SAFE,
            expr.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("`AssertSqlSafe` bypasses SQLx's SQL injection protection")
                    .help("keep SQL static and bind values, or use `QueryBuilder` for dynamic SQL");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
