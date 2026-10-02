#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks literal Insta filters whose replacement cannot normalize the match.
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
    pub INSTA_NOOP_FILTER,
    Warn,
    "an Insta snapshot filter replaces a literal with the same literal",
    InstaNoopFilter
}

impl<'tcx> LateLintPass<'tcx> for InstaNoopFilter {
    /// Compare literal patterns and replacements on resolved filter calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the filter method and require its exact two-argument shape.
        let Some(call) = settings_method_call(cx, expr, "add_filter") else {
            return;
        };
        let [pattern, replacement] = call.arguments else {
            return;
        };
        let (Some(pattern_value), Some(replacement_value)) =
            (string_literal(pattern), string_literal(replacement))
        else {
            return;
        };
        // Keep plain nonempty literals whose replacement preserves every match.
        if pattern_value.is_empty()
            || pattern_value != replacement_value
            || pattern_value
                .chars()
                .any(|character| r".^$*+?()[]{}|\".contains(character))
        {
            return;
        }

        cx.emit_span_lint(
            INSTA_NOOP_FILTER,
            expr.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this filter replaces matched text with itself")
                    .help("remove the filter or provide a stable replacement");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
