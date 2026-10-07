#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to preserve interpolated tracing message values as structured fields.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tracing as _;

use std::collections::HashSet;

use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use tracing_support::tracing_macro_invocation;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub TRACING_MESSAGE_INTERPOLATION,
    Warn,
    "tracing message interpolation hides values from structured fields",
    TracingMessageInterpolation,
    TracingMessageInterpolation::default()
}

/// Track macro invocations already reported from their expanded HIR.
/// The state prevents duplicate diagnostics when one source macro produces
/// several generated expressions with the same call-site span.
#[derive(Debug, Default)]
pub struct TracingMessageInterpolation {
    /// Complete macro spans that already emitted one diagnostic.
    reported: HashSet<Span>,
}

impl<'tcx> LateLintPass<'tcx> for TracingMessageInterpolation {
    /// Check each resolved tracing event macro once.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the macro and collect values used only inside its formatted message.
        let Some(invocation) = tracing_macro_invocation(cx, expr) else {
            return;
        };
        let values = invocation.unstructured_message_values().collect::<Vec<_>>();
        if values.is_empty() || !self.reported.insert(invocation.span) {
            return;
        }
        // Join all affected values into one structured-field remediation.
        let fields = values.join(", ");

        cx.emit_span_lint(
            TRACING_MESSAGE_INTERPOLATION,
            invocation.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("formatted values are only recorded in the tracing message")
                    .help(format!(
                        "record `{fields}` as structured fields before the message"
                    ));
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
