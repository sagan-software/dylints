#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for incompatible clap multicall and no-binary-name settings.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderCall, BuilderType, bool_argument, builder_calls, emit_lint_with_help,
    is_outermost_builder_call,
};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_MULTICALL_NO_BINARY_NAME,
    Warn,
    "`clap::Command` enables incompatible command-name parsing modes",
    ClapMulticallNoBinaryName
}

impl<'tcx> LateLintPass<'tcx> for ClapMulticallNoBinaryName {
    /// Check one expression for a complete clap command builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }
        let calls = builder_calls(cx, expr, BuilderType::Command);

        // Require both final settings to be the literal `true`.
        let (Some(true), Some(no_binary_name)) = (
            final_call(&calls, "multicall").and_then(bool_argument),
            final_call(&calls, "no_binary_name"),
        ) else {
            return;
        };
        if bool_argument(no_binary_name) != Some(true) {
            return;
        }
        emit_lint_with_help(
            cx,
            CLAP_MULTICALL_NO_BINARY_NAME,
            no_binary_name.span,
            "`multicall(true)` cannot be combined with `no_binary_name(true)`",
            "remove `no_binary_name(true)`; multicall mode already controls binary-name parsing",
        );
    }
}

/// Return the final call to one builder method, which replaces earlier calls.
fn final_call<'hir>(calls: &[BuilderCall<'hir>], name: &str) -> Option<BuilderCall<'hir>> {
    calls
        .iter()
        .rev()
        .find(|call| call.method.as_str() == name)
        .copied()
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
