#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `spawn_blocking` calls whose function returns a future.
//!
//! `tokio::task::spawn_blocking(f)` runs `f` on a blocking thread and returns
//! its result. When the result `R` is a future, nothing polls it: the task only
//! builds the future and hands it back. The lint reads `R` from the call's
//! inferred generic arguments and asks the trait solver whether it implements
//! `Future`, so async closures, closures that return async blocks, and plain
//! functions that return futures are all covered.

extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_trait_selection;

#[cfg(test)]
use tokio as _;

use rustc_hir::{Expr, ExprKind};
use rustc_infer::infer::TyCtxtInferExt as _;
use rustc_lint::{LateContext, LateLintPass};
use rustc_trait_selection::infer::InferCtxtExt as _;
use tokio_support::{emit, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_SPAWN_BLOCKING_ASYNC_CLOSURE,
    Warn,
    "spawn_blocking receives an async closure",
    TokioSpawnBlockingAsyncClosure
}

impl<'tcx> LateLintPass<'tcx> for TokioSpawnBlockingAsyncClosure {
    /// Check the resolved Tokio function and the type its argument returns.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some((span, _)) =
            tokio_function_call(cx, expr, "tokio::task::blocking::spawn_blocking")
        else {
            return;
        };
        if returns_future(cx, expr) {
            emit(
                cx,
                TOKIO_SPAWN_BLOCKING_ASYNC_CLOSURE,
                span,
                "the future returned by this closure is not polled",
                "use `tokio::spawn` or a synchronous closure",
                None,
            );
        }
    }
}

/// Return whether `spawn_blocking::<F, R>` is instantiated with a future `R`.
fn returns_future<'tcx>(cx: &LateContext<'tcx>, call: &Expr<'tcx>) -> bool {
    // Read the callee's inferred output type only for a direct function call.
    let ExprKind::Call(callee, _) = call.kind else {
        return false;
    };
    let Some(future) = cx.tcx.lang_items().future_trait() else {
        return false;
    };
    // The second generic argument is the result returned by `spawn_blocking`.
    let Some(output) = cx.typeck_results().node_args(callee.hir_id).types().nth(1) else {
        return false;
    };
    cx.tcx
        .infer_ctxt()
        .build(cx.typing_mode())
        .type_implements_trait(future, [output], cx.param_env)
        .must_apply_modulo_regions()
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
