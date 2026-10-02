#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to avoid allocating formatted strings for tracing fields.
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
use tracing_support::{
    TracingMacroInvocation, standard_format_invocation, tracing_macro_invocation,
};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub TRACING_FORMAT_FIELD,
    Warn,
    "format! allocates and hides structured values in a tracing field",
    TracingFormatField,
    TracingFormatField::default()
}

/// Collect resolved format calls and their enclosing tracing invocations.
///
/// The pass keeps deduplicated source spans so post-expansion analysis can join
/// each allocation with the tracing invocation that owns its structured field.
#[derive(Debug, Default)]
pub struct TracingFormatField {
    /// Complete tracing macro spans already added to `invocations`.
    seen_invocations: HashSet<Span>,
    /// Complete standard format macro spans already added to `format_calls`.
    seen_format_calls: HashSet<Span>,
    /// Source-level tracing invocations recovered from expanded HIR.
    invocations: Vec<TracingMacroInvocation>,
    /// Resolved standard format macro spans and their source.
    format_calls: Vec<(Span, String)>,
}

impl<'tcx> LateLintPass<'tcx> for TracingFormatField {
    /// Collect tracing and standard `format!` macro invocations.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(invocation) = tracing_macro_invocation(cx, expr)
            && self.seen_invocations.insert(invocation.span)
        {
            self.invocations.push(invocation);
        }

        if let Some(span) = standard_format_invocation(cx, expr)
            && self.seen_format_calls.insert(span)
            && let Ok(source) = cx.sess().source_map().span_to_snippet(span)
        {
            self.format_calls.push((span, source));
        }
    }

    /// Join source field grammar with resolved format macro calls after visiting the crate.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Associate recorded format calls with each enclosing tracing invocation.
        for invocation in &self.invocations {
            let fields: Vec<_> = self
                .format_calls
                .iter()
                .filter(|(span, _)| is_contained_by(invocation.span, *span))
                .filter_map(|(_, source)| invocation.formatted_field_for(source))
                .collect();
            if fields.is_empty() {
                continue;
            }
            // Emit one remediation containing every formatted field in the invocation.
            let fields = fields.join(", ");

            cx.emit_span_lint(
                TRACING_FORMAT_FIELD,
                invocation.span,
                DiagDecorator(move |diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("tracing field value is allocated with `format!`")
                        .help(format!(
                            "record the inputs to `{fields}` as separate fields, using `%` or `?` when needed"
                        ));
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
