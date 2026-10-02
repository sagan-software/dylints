#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Tokio unbounded channels.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
#[cfg(test)]
use tokio as _;
use tokio_support::tokio_function_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_UNBOUNDED_CHANNEL,
    Warn,
    "a Tokio channel is unbounded",
    TokioUnboundedChannel
}

impl<'tcx> LateLintPass<'tcx> for TokioUnboundedChannel {
    /// Check the resolved Tokio channel constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some((span, _)) =
            tokio_function_call(cx, expr, "tokio::sync::mpsc::unbounded::unbounded_channel")
        else {
            return;
        };
        cx.emit_span_lint(
            TOKIO_UNBOUNDED_CHANNEL,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this channel can buffer until memory is exhausted")
                    .help("use `mpsc::channel` with an explicit capacity");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
