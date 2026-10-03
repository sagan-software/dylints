#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for async blocks passed to synchronous tracing scopes.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves tracing scope calls, reports async blocks in
//! synchronous scopes, and recommends an async-compatible instrumentation path.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use tracing_support::{call_returns_future, tracing_function_arguments, tracing_method_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_ASYNC_BLOCK_IN_SYNC_SCOPE,
    Warn,
    "an async block is returned from a synchronous tracing scope",
    TracingAsyncBlockInSyncScope
}

impl<'tcx> LateLintPass<'tcx> for TracingAsyncBlockInSyncScope {
    /// Check futures returned from tracing's synchronous context APIs.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Check the exact span method and its returned type to accept every callable form.
        if let Some(method) = tracing_method_call(cx, expr, "tracing::span::Span::in_scope")
            && let [_callable] = method.arguments
            && call_returns_future(cx, expr)
        {
            emit_async_scope_lint(
                cx,
                method.method_span,
                "this span scope ends before the returned future is polled",
                "attach the span with `Future::instrument` instead",
            );
            return;
        }

        // Resolve both subscriber scope functions before checking their returned type.
        let Some(arguments) = tracing_function_arguments(
            cx,
            expr,
            &[
                "tracing::subscriber::with_default",
                "tracing_core::dispatcher::with_default",
            ],
        ) else {
            return;
        };
        let [_, _callable] = arguments else {
            return;
        };
        if call_returns_future(cx, expr) {
            emit_async_scope_lint(
                cx,
                expr.span,
                "this subscriber scope ends before the returned future is polled",
                "attach the subscriber with `WithSubscriber::with_subscriber` instead",
            );
        }
    }
}

/// Emit a synchronous-scope diagnostic with the matching async alternative.
fn emit_async_scope_lint(
    cx: &LateContext<'_>,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        TRACING_ASYNC_BLOCK_IN_SYNC_SCOPE,
        span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic.primary_message(message).help(help);
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
