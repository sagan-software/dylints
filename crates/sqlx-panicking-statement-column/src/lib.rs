#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks panicking `SQLx` column access on statements and rows.
//!
//! This Dylint library resolves calls to `Statement::column` and
//! `Row::column` through type-dependent lookup, so a local method with the same
//! name does not match. Both methods panic for an index or name that does not
//! match a column, and both have a `try_column` counterpart that returns an
//! error instead.

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
    pub SQLX_PANICKING_STATEMENT_COLUMN,
    Warn,
    "SQLx Statement::column and Row::column can panic",
    SqlxPanickingStatementColumn
}

impl<'tcx> LateLintPass<'tcx> for SqlxPanickingStatementColumn {
    /// Check semantically resolved calls to a panicking `column` method.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(method) = sqlx_method_call(cx, expr, &["Statement", "Row"], &["column"]) else {
            return;
        };
        cx.emit_span_lint(
            SQLX_PANICKING_STATEMENT_COLUMN,
            method.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("SQLx `column` panics for an invalid index or name")
                    .help("use `try_column` and handle or propagate the error");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
