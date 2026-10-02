#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for panicking `SQLx` row access.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves `SQLx` row accessors, reports unchecked panics,
//! and recommends handling the database conversion result explicitly.

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
    pub SQLX_PANICKING_ROW_GET,
    Warn,
    "SQLx row access can panic",
    SqlxPanickingRowGet
}

impl<'tcx> LateLintPass<'tcx> for SqlxPanickingRowGet {
    /// Check semantically resolved calls to `SQLx`'s panicking row methods.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Exclude generated calls before resolving SQLx row access methods.
        if expr.span.from_expansion() {
            return;
        }
        let Some(method) = sqlx_method_call(cx, expr, &["Row"], &["get", "get_unchecked"]) else {
            return;
        };
        // Select the matching fallible method for the resolved panicking variant.
        let replacement = if method.name.as_str() == "get" {
            "try_get"
        } else {
            "try_get_unchecked"
        };

        cx.emit_span_lint(
            SQLX_PANICKING_ROW_GET,
            method.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message(format!("SQLx `Row::{}` can panic", method.name))
                    .help(format!(
                        "use `Row::{replacement}` and handle or propagate the decoding error"
                    ));
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
