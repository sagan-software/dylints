#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for `Router::route_layer` on a newly created Axum router.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use axum as _;

use axum_support::{is_router_new_call, router_method_call};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AXUM_ROUTE_LAYER_ON_EMPTY_ROUTER,
    Warn,
    "`Router::route_layer` is called on a newly created Axum router",
    AxumRouteLayerOnEmptyRouter
}

impl<'tcx> LateLintPass<'tcx> for AxumRouteLayerOnEmptyRouter {
    /// Prove the receiver is Axum's empty `Router::new()` value.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the layer method and require its receiver to be a fresh empty router.
        let Some(call) = router_method_call(cx, expr, "route_layer") else {
            return;
        };
        if !is_router_new_call(cx, call.receiver) {
            return;
        }

        cx.emit_span_lint(
            AXUM_ROUTE_LAYER_ON_EMPTY_ROUTER,
            call.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("applying a route layer before adding routes panics")
                    .help("add the routes before calling `Router::route_layer`");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
