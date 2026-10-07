#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for custom Reqwest cookie providers that are overwritten.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, is_true_literal, reqwest_method_parts};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_COOKIE_PROVIDER_OVERRIDDEN,
    Warn,
    "a custom Reqwest cookie provider is overwritten by the default store",
    ReqwestCookieProviderOverridden
}

impl<'tcx> LateLintPass<'tcx> for ReqwestCookieProviderOverridden {
    /// Check one `cookie_store(true)` call and its fluent receiver chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Require an enabled default store after an earlier custom provider.
        let Some((receiver, [enabled], span)) = reqwest_method_parts(cx, expr, "cookie_store")
        else {
            return;
        };
        if !is_true_literal(enabled) || !chain_has_cookie_provider(cx, receiver) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_COOKIE_PROVIDER_OVERRIDDEN,
            span,
            "`cookie_store(true)` replaces the custom cookie provider set earlier in this chain",
            "remove `cookie_store(true)`, or choose only the default store",
        );
    }
}

/// Follow a fluent Reqwest builder chain looking for `cookie_provider`.
fn chain_has_cookie_provider(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Walk receiver links backward until the provider call or chain root is reached.
    if reqwest_method_parts(cx, expr, "cookie_provider").is_some() {
        return true;
    }

    let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
        return false;
    };
    chain_has_cookie_provider(cx, receiver)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
