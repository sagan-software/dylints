#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for disabled Reqwest certificate validation.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Reqwest TLS settings, reports disabled
//! certificate validation, and recommends leaving certificate checks enabled.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, is_true_boolean_expression, reqwest_method_parts};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_TLS_DANGER_INVALID_CERTS,
    Warn,
    "Reqwest TLS certificate validation is disabled",
    ReqwestTlsDangerInvalidCerts
}

impl<'tcx> LateLintPass<'tcx> for ReqwestTlsDangerInvalidCerts {
    /// Check the current and soft-deprecated certificate-validation methods.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Normalize both supported method spellings to one resolved call shape.
        let call = reqwest_method_parts(cx, expr, "tls_danger_accept_invalid_certs")
            .or_else(|| reqwest_method_parts(cx, expr, "danger_accept_invalid_certs"));
        let Some((_receiver, [enabled], span)) = call else {
            return;
        };
        // Report only a setting proven to disable certificate validation.
        if !is_true_boolean_expression(cx, enabled) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_TLS_DANGER_INVALID_CERTS,
            span,
            "this client trusts invalid certificates for every site",
            "keep validation enabled and configure explicit certificate roots",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
