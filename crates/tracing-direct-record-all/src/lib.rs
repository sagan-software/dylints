#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for direct calls to tracing's hidden record-all method.
//!
//! This Dylint library resolves tracing span calls, reports direct hidden
//! record-all operations, and recommends the public field-recording interface.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use tracing_support::tracing_method_call;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_DIRECT_RECORD_ALL,
    Warn,
    "tracing Span::record_all is an internal API",
    TracingDirectRecordAll
}

impl<'tcx> LateLintPass<'tcx> for TracingDirectRecordAll {
    /// Check user-written calls while ignoring the supported macro's expansion.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Exclude macro expansions before resolving the internal method call.
        if expr.span.from_expansion() {
            return;
        }
        let Some(method) = tracing_method_call(cx, expr, "tracing::span::Span::record_all") else {
            return;
        };

        cx.emit_span_lint(
            TRACING_DIRECT_RECORD_ALL,
            method.method_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("direct `Span::record_all` calls use a hidden tracing API")
                    .help("use the supported `tracing::record_all!` macro");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
