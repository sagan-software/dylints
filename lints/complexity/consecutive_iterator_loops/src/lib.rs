#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for consecutive iterator loops that can use `Iterator::chain`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{Block, Expr};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CONSECUTIVE_ITERATOR_LOOPS,
    Warn,
    "consecutive iterator loops can use `Iterator::chain`",
    ConsecutiveIteratorLoops
}

impl<'tcx> LateLintPass<'tcx> for ConsecutiveIteratorLoops {
    /// Checks adjacent standard loops with inert sources and identical bodies.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Check statement pairs first, then the common statement-plus-tail form.
        for pair in block.stmts.windows(2) {
            let [first_stmt, second_stmt] = pair else {
                continue;
            };
            let (Some(first_expr), Some(second_expr)) = (
                support::stmt_expr(first_stmt),
                support::stmt_expr(second_stmt),
            ) else {
                continue;
            };
            check_pair(cx, first_expr, second_expr);
        }

        // Include a final expression paired with the preceding statement.
        if let (Some(last), Some(tail)) = (block.stmts.last(), block.expr)
            && let Some(first_expr) = support::stmt_expr(last)
        {
            check_pair(cx, first_expr, tail);
        }
    }
}

/// Checks one adjacent expression pair and emits for two compatible loops.
fn check_pair<'tcx>(
    cx: &LateContext<'tcx>,
    first_expr: &'tcx Expr<'tcx>,
    second_expr: &'tcx Expr<'tcx>,
) {
    if let Some(span) = chainable_loop_span(cx, first_expr, second_expr) {
        support::emit(
            cx,
            CONSECUTIVE_ITERATOR_LOOPS,
            span,
            "consecutive iterator loops can use `Iterator::chain`",
            "use `Iterator::chain` and apply the shared body once",
        );
    }
}

/// Return the combined span for two loops with inert sources and equal bodies.
fn chainable_loop_span<'tcx>(
    cx: &LateContext<'tcx>,
    first_expr: &'tcx Expr<'tcx>,
    second_expr: &'tcx Expr<'tcx>,
) -> Option<rustc_span::Span> {
    // Require two semantic loops before comparing their restricted source shapes.
    support::for_loop(cx, first_expr)
        .zip(support::for_loop(cx, second_expr))
        .and_then(|(first, second)| {
            support::snippet(cx, first.source.span).and_then(|first_source| {
                support::snippet(cx, second.source.span)
                    .map(|second_source| (first, second, first_source, second_source))
            })
        })
        // Restrict chaining to inert sources whose evaluation timing is unchanged.
        .filter(|(_, _, first_source, second_source)| {
            inert_source(first_source) && inert_source(second_source)
        })
        // Compare exact user-written bodies before proposing one shared closure.
        .and_then(|(first, second, _, _)| {
            support::snippet(cx, first.body.span).and_then(|first_body| {
                support::snippet(cx, second.body.span)
                    .filter(|second_body| *second_body == first_body)
                    .map(|_| first.span.to(second.span))
            })
        })
}

/// Returns whether a source is a local path or simple borrow of one.
fn inert_source(source: &str) -> bool {
    // Restrict conversion timing changes to a local name or one direct borrow.
    let source = source.trim();
    let source = source
        .strip_prefix('&')
        .unwrap_or(source)
        .trim_start_matches("mut ")
        .trim();
    !source.is_empty()
        && source
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
