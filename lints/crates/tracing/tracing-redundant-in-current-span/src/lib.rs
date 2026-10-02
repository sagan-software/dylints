#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for redundant tracing future instrumentation.
//!
//! This Dylint library resolves tracing future instrumentation, reports
//! redundant current-span setup, and recommends the direct instrumentation form.
//! The README and UI fixtures define the supported boundary, replacement, and
//! non-triggering cases so callers can adopt the diagnostic safely.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use tracing_support::tracing_method_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_REDUNDANT_IN_CURRENT_SPAN,
    Warn,
    "a tracing future is redundantly wrapped in the current span",
    TracingRedundantInCurrentSpan
}

impl<'tcx> LateLintPass<'tcx> for TracingRedundantInCurrentSpan {
    /// Check the exact method nesting documented by tracing.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the outer span attachment before comparing its instrumented receiver.
        let Some(outer) =
            tracing_method_call(cx, expr, "tracing::instrument::Instrument::in_current_span")
        else {
            return;
        };
        if tracing_method_call(
            cx,
            outer.receiver,
            "tracing::instrument::Instrument::instrument",
        )
        .is_none()
        {
            return;
        }

        cx.emit_span_lint(
            TRACING_REDUNDANT_IN_CURRENT_SPAN,
            outer.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this nested tracing instrumentation is redundant")
                    .help("pass `span.or_current()` to `instrument` and remove `in_current_span`");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
