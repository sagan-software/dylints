#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero-capacity Tokio channels.
//!
//! The lint resolves calls to the bounded `mpsc` and `broadcast` channel
//! constructors and reports a literal zero capacity, which makes Tokio panic.
//! It leaves computed capacities alone because their runtime value is not known statically.

extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use tokio_support::{emit, is_zero_integer, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_ZERO_CAPACITY_CHANNEL,
    Warn,
    "a Tokio channel is constructed with zero capacity",
    TokioZeroCapacityChannel
}

/// The definition paths of Tokio constructors that reject a zero capacity.
const CHANNEL_PATHS: [&str; 2] = [
    "tokio::sync::broadcast::channel",
    "tokio::sync::mpsc::bounded::channel",
];

impl<'tcx> LateLintPass<'tcx> for TokioZeroCapacityChannel {
    /// Check one call to a bounded Tokio channel constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let capacity = CHANNEL_PATHS
            .iter()
            .find_map(|path| tokio_function_call(cx, expr, path))
            .and_then(|(_, arguments)| arguments.first());
        if let Some(capacity) = capacity
            && is_zero_integer(capacity)
        {
            emit(
                cx,
                TOKIO_ZERO_CAPACITY_CHANNEL,
                capacity.span,
                "Tokio channel capacity must be greater than zero",
                "use a positive capacity",
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
