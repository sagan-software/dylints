#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks literal Insta filters whose replacement cannot normalize the match.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use insta_support::{settings_method_call, string_literal};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Expr, Node, StmtKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_NOOP_FILTER,
    Warn,
    "an Insta snapshot filter replaces a literal with the same literal",
    InstaNoopFilter
}

impl<'tcx> LateLintPass<'tcx> for InstaNoopFilter {
    /// Compare literal patterns and replacements on resolved filter calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the filter method and require its exact two-argument shape.
        let Some(call) = settings_method_call(cx, expr, "add_filter") else {
            return;
        };
        let [pattern, replacement] = call.arguments else {
            return;
        };
        let (Some(pattern_value), Some(replacement_value)) =
            (string_literal(cx, pattern), string_literal(cx, replacement))
        else {
            return;
        };
        // Keep plain nonempty literals whose replacement preserves every match.
        if pattern_value.is_empty()
            || pattern_value != replacement_value
            || pattern_value
                .chars()
                .any(|character| r".^$*+?()[]{}|\".contains(character))
        {
            return;
        }

        let statement_span = statement_span(cx, expr);
        cx.emit_span_lint(
            INSTA_NOOP_FILTER,
            expr.span,
            DiagDecorator(move |diagnostic| {
                let diagnostic =
                    diagnostic.primary_message("this filter replaces matched text with itself");
                // Removing a whole statement drops only a filter that changes nothing.
                if let Some(statement_span) = statement_span {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        statement_span,
                        "remove the filter",
                        "",
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help =
                        diagnostic.help("remove the filter or provide a stable replacement");
                }
            }),
        );
    }
}

/// Return the span of the `expr;` statement that consists of this call alone.
fn statement_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    let (_, Node::Stmt(statement)) = cx.tcx.hir_parent_iter(expr.hir_id).next()? else {
        return None;
    };
    // Preserve help-only diagnostics for compiler-generated statements.
    if !matches!(statement.kind, StmtKind::Semi(_)) || statement.span.from_expansion() {
        return None;
    }
    // Include indentation only when it is the complete source prefix on this line.
    let source_map = cx.sess().source_map();
    let line_start = source_map
        .lookup_source_file(statement.span.lo())
        .line_begin_pos(statement.span.lo());
    let line_prefix = statement.span.with_lo(line_start);
    let leading_whitespace = line_prefix.with_hi(statement.span.lo());
    source_map
        .span_to_snippet(leading_whitespace)
        .ok()
        .filter(|prefix| prefix.chars().all(char::is_whitespace))
        .map_or(Some(statement.span), |_| Some(line_prefix))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
