#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Tokio `blocking_*` synchronization methods called in async code.
//!
//! The lint resolves each method call to its Tokio definition and reports the
//! blocking methods that panic inside an asynchronous execution context. It
//! suggests the matching `.await` form, which callers must review because the
//! enclosing future then holds the new await point.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use tokio as _;

use rustc_errors::Applicability;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use tokio_support::{TokioMethod, emit, is_in_async_body, tokio_method};

/// Async equivalents for Tokio's blocking method names.
const ASYNC_REPLACEMENTS: &[(&str, &str)] = &[
    ("blocking_lock", "lock"),
    ("blocking_lock_owned", "lock_owned"),
    ("blocking_read", "read"),
    ("blocking_write", "write"),
    ("blocking_recv", "recv"),
    ("blocking_recv_many", "recv_many"),
    ("blocking_send", "send"),
];

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
        // Resolve the Tokio method before inspecting its asynchronous context.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        // Keep only methods with a known asynchronous equivalent.
        let Some(replacement) = async_replacement(&method) else {
            return;
        };
        if !is_in_async_body(cx, expr) {
            return;
        }
        // Rewrite from the method name to the end of the call, keeping the arguments.
        let span = expr.span.with_lo(method.span.lo());
        let arguments = cx
            .sess()
            .source_map()
            .span_to_snippet(expr.span.with_lo(method.span.hi()))
            .ok();
        let suggestion = arguments.map(|arguments| {
            let replacement = if replacement.is_empty() {
                "await".to_owned()
            } else {
                format!("{replacement}{arguments}.await")
            };
            (span, replacement, Applicability::MaybeIncorrect)
        });
        emit(
            cx,
            TOKIO_BLOCKING_CALL_IN_ASYNC,
            method.span,
            "this Tokio blocking method panics in an asynchronous execution context",
            "use the asynchronous equivalent with `.await`, or call it inside `spawn_blocking`",
            suggestion,
        );
    }
}

/// Return the async method that replaces a blocking one, or `""` to await the receiver.
fn async_replacement(method: &TokioMethod) -> Option<&'static str> {
    // A oneshot receiver is itself the future that `blocking_recv` waits for.
    if method.definition_name == "tokio::sync::oneshot::Receiver::blocking_recv" {
        return Some("");
    }
    ASYNC_REPLACEMENTS
        .iter()
        .find_map(|(blocking, asynchronous)| {
            (method.name.as_str() == *blocking).then_some(*asynchronous)
        })
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
