#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for clap positional-only last settings on options.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderType, bool_argument, builder_calls, emit_lint_with_help, has_option_name,
    is_outermost_builder_call,
};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_LAST_OPTION,
    Warn,
    "`clap::Arg::last` is enabled on an option",
    ClapLastOption
}

impl<'tcx> LateLintPass<'tcx> for ClapLastOption {
    /// Check one expression for a complete clap argument builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }
        let calls = builder_calls(cx, expr, BuilderType::Arg);

        // Require an enabled final `last` setting and at least one option name.
        let Some(last) = calls
            .iter()
            .rev()
            .find(|call| call.method.as_str() == "last")
        else {
            return;
        };
        if bool_argument(*last) != Some(true) || !has_option_name(cx, &calls) {
            return;
        }

        // Anchor the conflict at the final enabled `last` setting.
        emit_lint_with_help(
            cx,
            CLAP_LAST_OPTION,
            last.span,
            "`last(true)` has no effect on a clap option",
            "remove `last(true)`, or make the argument positional by removing its long and short names",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
