#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for invalid Reqwest retry budget percentages.
//!
//! This Dylint library resolves Reqwest retry settings, reports invalid extra
//! load percentages, and recommends a value within the supported range.
//!
//! The README defines the finite boundary and diagnostic replacement. UI
//! fixtures cover statically known values and unknown runtime inputs.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, reqwest_f32_constant, reqwest_method_parts};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_RETRY_INVALID_MAX_EXTRA_LOAD,
    Warn,
    "a Reqwest retry budget percentage will panic",
    ReqwestRetryInvalidMaxExtraLoad
}

impl<'tcx> LateLintPass<'tcx> for ReqwestRetryInvalidMaxExtraLoad {
    /// Check statically known f32 retry-budget percentages against Reqwest's panic bounds.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact one-argument retry policy method before inspecting its value.
        let Some((_receiver, [extra_load], span)) =
            reqwest_method_parts(cx, expr, "max_extra_load")
        else {
            return;
        };
        // Evaluate only literals, local constants, and bounded pure f32 arithmetic.
        let Some(value) = reqwest_f32_constant(cx, extra_load) else {
            return;
        };
        if (0.0..=1000.0).contains(&value) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_RETRY_INVALID_MAX_EXTRA_LOAD,
            span,
            "Reqwest panics when `max_extra_load` is outside the inclusive 0.0 through 1000.0 range",
            "use a finite value in the inclusive range 0.0 through 1000.0",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
