#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks direct matching on Insta `Content` before resolving internal wrappers.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;

use insta_support::{content_method_call, is_content_expression};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_CONTENT_DIRECT_MATCH,
    Warn,
    "Insta Content is pattern matched without resolving internal wrappers",
    InstaContentDirectMatch
}

impl<'tcx> LateLintPass<'tcx> for InstaContentDirectMatch {
    /// Check match scrutinees with the resolved Insta `Content` type.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Report direct matches on snapshot content while allowing derived predicates.
        let ExprKind::Match(scrutinee, _, _) = expr.kind else {
            return;
        };
        if !is_content_expression(cx, scrutinee)
            || content_method_call(cx, scrutinee, "resolve_inner").is_some()
        {
            return;
        }

        cx.emit_span_lint(
            INSTA_CONTENT_DIRECT_MATCH,
            scrutinee.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("direct matching can miss Insta's internal wrapper variants")
                    .help("use a `Content::as_*` accessor, or match on `resolve_inner()`");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
