#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Tokio blocking APIs in async bodies.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Tokio calls in async bodies, reports a source
//! diagnostic for blocking operations, and recommends an async replacement.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tokio as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;
use tokio_support::{is_in_async_body, tokio_method};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_BLOCKING_CALL_IN_ASYNC,
    Warn,
    "a Tokio blocking API is called in an async body",
    TokioBlockingCallInAsync
}

impl<'tcx> LateLintPass<'tcx> for TokioBlockingCallInAsync {
    /// Check one resolved Tokio method call in its nearest closure boundary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Require a known blocking method inside the nearest async execution boundary.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        if !matches!(
            method.name.as_str(),
            "blocking_lock"
                | "blocking_lock_owned"
                | "blocking_read"
                | "blocking_read_owned"
                | "blocking_recv"
                | "blocking_send"
                | "blocking_write"
                | "blocking_write_owned"
        ) || !is_in_async_body(cx, expr)
        {
            return;
        }

        emit_span_lint_with_help(
            cx,
            TOKIO_BLOCKING_CALL_IN_ASYNC,
            method.span,
            "this Tokio blocking method panics in an asynchronous execution context",
            "use the asynchronous equivalent with `.await`, or call it inside `spawn_blocking`",
        );
    }
}

/// Emit the diagnostic with Tokio's documented constraint.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        lint,
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
