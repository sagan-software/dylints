#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Tokio unbounded channels.
//!
//! The lint resolves calls to `tokio::sync::mpsc::unbounded_channel` and
//! recommends a bounded channel with an explicit capacity. Bounded queues make
//! backpressure visible at the call site and limit retained messages. Callers
//! then wait when the queue reaches its configured capacity.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, tokio_function_call};

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
        if let Some((span, _)) =
            tokio_function_call(cx, expr, "tokio::sync::mpsc::unbounded::unbounded_channel")
        {
            emit(
                cx,
                TOKIO_UNBOUNDED_CHANNEL,
                span,
                "this channel can buffer until memory is exhausted",
                "use `mpsc::channel` with an explicit capacity",
                None,
            );
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
