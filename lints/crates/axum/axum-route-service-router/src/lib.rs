#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Axum routers passed to `Router::route_service`.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use axum as _;

use axum_support::{is_router_type, router_method_call};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AXUM_ROUTE_SERVICE_ROUTER,
    Warn,
    "an Axum router is passed to `Router::route_service`",
    AxumRouteServiceRouter
}

impl<'tcx> LateLintPass<'tcx> for AxumRouteServiceRouter {
    /// Check the resolved service type rather than the source path used to construct it.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact router method and its service argument before type checking.
        let Some(call) = router_method_call(cx, expr, "route_service") else {
            return;
        };
        // Report only service values whose adjusted type is another Axum router.
        if !call
            .arguments
            .last()
            .is_some_and(|service| is_router_type(cx, service))
        {
            return;
        }

        cx.emit_span_lint(
            AXUM_ROUTE_SERVICE_ROUTER,
            call.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("routing to another `Router` here panics")
                    .help("compose the inner router with `Router::nest` instead");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
