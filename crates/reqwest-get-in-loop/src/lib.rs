#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Reqwest GET shortcuts in loops.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, is_in_loop, reqwest_function};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_GET_IN_LOOP,
    Warn,
    "a Reqwest get shortcut creates a client inside a loop",
    ReqwestGetInLoop
}

impl<'tcx> LateLintPass<'tcx> for ReqwestGetInLoop {
    /// Check a resolved Reqwest free-function call inside a lexical loop.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the free function before comparing supported async and blocking paths.
        let Some(function) = reqwest_function(cx, expr) else {
            return;
        };
        if function.name.as_str() != "get" {
            return;
        }
        let is_reqwest_get = function.definition == "reqwest::get"
            || function.definition.contains("::blocking::get");
        if !is_reqwest_get {
            return;
        }
        // Report only when the resolved shortcut executes inside a lexical loop.
        if !is_in_loop(cx, expr) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_GET_IN_LOOP,
            function.span,
            "this Reqwest shortcut creates a new client on every loop iteration",
            "create one `reqwest::Client` before the loop and call `client.get(...)`",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
