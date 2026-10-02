#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for futures returned from synchronous Insta settings bindings.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Insta settings bindings, reports futures held
//! across synchronous calls, and recommends awaiting them before binding.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;

use insta_support::{is_explicit_async_closure, settings_method_call};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SETTINGS_BIND_FUTURE,
    Warn,
    "a future is returned from a synchronous Insta settings binding",
    InstaSettingsBindFuture
}

impl<'tcx> LateLintPass<'tcx> for InstaSettingsBindFuture {
    /// Check explicit futures returned from semantically resolved `Settings::bind` calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact bind method and require its sole closure argument.
        let Some(call) = settings_method_call(cx, expr, "bind") else {
            return;
        };
        let [closure] = call.arguments else {
            return;
        };
        // Report only closures whose result is an explicit future.
        if !is_explicit_async_closure(cx, closure) {
            return;
        }

        cx.emit_span_lint(
            INSTA_SETTINGS_BIND_FUTURE,
            call.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("these settings reset before the returned future is polled")
                    .span_suggestion(
                        call.method_span,
                        "pass the async block directly to `Settings::bind_async` instead",
                        "bind_async",
                        Applicability::MachineApplicable,
                    );
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
