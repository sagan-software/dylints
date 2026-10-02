#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty regular-expression patterns in Insta snapshot filters.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;

use insta_support::{settings_method_call, string_literal};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_EMPTY_FILTER_PATTERN,
    Warn,
    "an Insta snapshot filter has an empty regular expression",
    InstaEmptyFilterPattern
}

impl<'tcx> LateLintPass<'tcx> for InstaEmptyFilterPattern {
    /// Check the literal pattern on resolved `Settings::add_filter` calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve only the target Settings method and its two expected arguments.
        let Some(call) = settings_method_call(cx, expr, "add_filter") else {
            return;
        };
        let [pattern, _replacement] = call.arguments else {
            return;
        };

        // Report only the literal empty pattern that matches every position.
        if string_literal(pattern).as_deref() != Some("") {
            return;
        }

        cx.emit_span_lint(
            INSTA_EMPTY_FILTER_PATTERN,
            pattern.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this empty regex matches every position in the snapshot")
                    .help("use a regex that selects only the unstable snapshot content");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
