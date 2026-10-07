#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to avoid allocating strings for tracing fields.
//!
//! This Dylint library resolves tracing field calls, reports unnecessary string
//! allocations, and recommends passing the original value directly.
//!
//! The README defines supported field calls and the replacement. UI fixtures
//! cover triggering and non-triggering forms so unrelated code remains unchanged.

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
use tracing_support::{TracingMacroInvocation, is_to_string_trait_call, tracing_macro_invocation};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub TRACING_TO_STRING_FIELD,
    Warn,
    "to_string allocates before a value is recorded as a tracing field",
    TracingToStringField,
    TracingToStringField::default()
}

/// Collect resolved calls and their enclosing tracing invocations.
///
/// The pass deduplicates macro spans and retains source-level method calls so
/// post-expansion analysis can report one remediation for each tracing event.
#[derive(Debug, Default)]
pub struct TracingToStringField {
    /// Complete tracing macro spans already added to `invocations`.
    seen_invocations: HashSet<Span>,
    /// Source-level tracing invocations recovered from expanded HIR.
    invocations: Vec<TracingMacroInvocation>,
    /// Resolved `ToString` method spans and their source.
    to_string_calls: Vec<(Span, String)>,
}

impl<'tcx> LateLintPass<'tcx> for TracingToStringField {
    /// Collect tracing invocations and semantically resolved `ToString` calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(invocation) = tracing_macro_invocation(cx, expr)
            && self.seen_invocations.insert(invocation.span)
        {
            self.invocations.push(invocation);
        }

        if is_to_string_trait_call(cx, expr)
            && let Ok(source) = cx.sess().source_map().span_to_snippet(expr.span)
        {
            self.to_string_calls.push((expr.span, source));
        }
    }

    /// Join source field grammar with resolved method calls after visiting the crate.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Associate recorded `to_string` calls with each enclosing tracing invocation.
        for invocation in &self.invocations {
            let fields: Vec<_> = self
                .to_string_calls
                .iter()
                .filter(|(span, _)| is_contained_by(invocation.span, *span))
                .filter_map(|(_, source)| invocation.stringified_field_for(source))
                .collect();
            if fields.is_empty() {
                continue;
            }
            // Emit one remediation containing every allocating field in the invocation.
            let fields = fields.join(", ");

            cx.emit_span_lint(
                TRACING_TO_STRING_FIELD,
                invocation.span,
                DiagDecorator(move |diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("tracing field value is allocated with `.to_string()`")
                        .help(format!("record `{fields}` with tracing's `%` sigil"));
                }),
            );
        }
    }
}

/// Return whether one source span encloses another.
fn is_contained_by(outer: Span, inner: Span) -> bool {
    outer.lo() <= inner.lo() && inner.hi() <= outer.hi()
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
