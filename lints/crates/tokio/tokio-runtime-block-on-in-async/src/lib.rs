#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `Runtime::block_on` inside async code.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
#[cfg(test)]
use tokio as _;
use tokio_support::{is_in_async_body, tokio_method};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_RUNTIME_BLOCK_ON_IN_ASYNC,
    Warn,
    "Tokio Runtime::block_on is called in async code",
    TokioRuntimeBlockOnInAsync
}

impl<'tcx> LateLintPass<'tcx> for TokioRuntimeBlockOnInAsync {
    /// Check a resolved runtime method in its nearest closure boundary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact runtime method before checking async ancestry.
        let Some(method) = tokio_method(cx, expr) else {
            return;
        };
        if method.name.as_str() != "block_on" {
            return;
        }
        if !method.definition.contains("Runtime::block_on") {
            return;
        }
        // A synchronous body may use `block_on` without this nested-runtime hazard.
        if !is_in_async_body(cx, expr) {
            return;
        }
        cx.emit_span_lint(
            TOKIO_RUNTIME_BLOCK_ON_IN_ASYNC,
            method.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("`Runtime::block_on` can panic inside a Tokio runtime")
                    .help("await the future directly");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
