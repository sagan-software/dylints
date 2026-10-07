#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Reqwest client construction in loops.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Reqwest client constructors, reports repeated
//! construction in loops, and recommends reusing one configured client.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, is_in_loop, reqwest_function};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_CLIENT_IN_LOOP,
    Warn,
    "a Reqwest client is constructed inside a loop",
    ReqwestClientInLoop
}

impl<'tcx> LateLintPass<'tcx> for ReqwestClientInLoop {
    /// Check direct Reqwest client constructors inside lexical loops.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the constructor and require lexical loop ancestry before reporting.
        let Some(function) = reqwest_function(cx, expr) else {
            return;
        };
        if !is_client_constructor(&function.definition, function.name.as_str())
            || !is_in_loop(cx, expr)
        {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_CLIENT_IN_LOOP,
            function.span,
            "this loop constructs a new Reqwest connection pool on every iteration",
            "construct the client before the loop and reuse it",
        );
    }
}

/// Recognize the documented async and blocking client constructors.
fn is_client_constructor(path: &str, name: &str) -> bool {
    matches!(name, "new" | "builder")
        && (path.contains("Client::new")
            || path.contains("Client::builder")
            || path.contains("ClientBuilder::new"))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
