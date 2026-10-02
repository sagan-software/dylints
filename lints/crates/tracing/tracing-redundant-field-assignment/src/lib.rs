#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to use tracing's structured field shorthand.
//!
//! This Dylint library resolves tracing field assignments, reports repeated
//! paths, and recommends structured-field shorthand where source permits it.
//!
//! The README defines the supported macro shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use tracing as _;

use std::collections::HashSet;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use tracing_support::tracing_macro_invocation;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub TRACING_REDUNDANT_FIELD_ASSIGNMENT,
    Warn,
    "tracing supports shorthand for fields assigned from the same path",
    TracingRedundantFieldAssignment,
    TracingRedundantFieldAssignment::default()
}

/// Track macro invocations already reported from their expanded HIR.
///
/// The pass stores only complete invocation spans, so repeated expansion visits
/// produce one diagnostic without retaining user source or configuration.
#[derive(Debug, Default)]
pub struct TracingRedundantFieldAssignment {
    /// Complete macro spans that already emitted one diagnostic.
    reported: HashSet<Span>,
}

impl<'tcx> LateLintPass<'tcx> for TracingRedundantFieldAssignment {
    /// Check each resolved tracing field macro once.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the macro invocation and collect its redundant field assignments.
        let Some(invocation) = tracing_macro_invocation(cx, expr) else {
            return;
        };
        let fields = invocation.redundant_field_assignments().collect::<Vec<_>>();
        if fields.is_empty() || !self.reported.insert(invocation.span) {
            return;
        }
        // Join all shorthand names into one remediation for the complete macro span.
        let shorthand = fields.join(", ");
        let replacement = invocation.redundant_field_assignment_replacement();

        cx.emit_span_lint(
            TRACING_REDUNDANT_FIELD_ASSIGNMENT,
            invocation.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic =
                    diagnostic.primary_message("tracing field assignment repeats the same path");
                if let Some(replacement) = replacement {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        invocation.span,
                        format!("use tracing field shorthand: `{shorthand}`"),
                        replacement,
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help =
                        diagnostic.help(format!("use tracing field shorthand: `{shorthand}`"));
                }
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
