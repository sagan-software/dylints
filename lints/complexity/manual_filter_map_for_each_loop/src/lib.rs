#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

//! A lint for loops that can use `filter_map(...).for_each(...)`.
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

use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_FILTER_MAP_FOR_EACH_LOOP,
    Warn,
    "conditional mapping loop can use `filter_map(...).for_each(...)`",
    ManualFilterMapForEachLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualFilterMapForEachLoop {
    /// Checks one standard loop for a complete `if let Some` body.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(span) = filter_map_loop_span(cx, expr) {
            support::emit(
                cx,
                MANUAL_FILTER_MAP_FOR_EACH_LOOP,
                span,
                "conditional mapping loop can use `filter_map(...).for_each(...)`",
                "use `filter_map(...).for_each(...)` for this conditional action",
            );
        }
    }
}

/// Return the span of a loop whose body is one safe `Option` mapping action.
fn filter_map_loop_span<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    // Recover the loop, then preserve the source checks that make the adapter equivalent.
    (!matches!(expr.kind, ExprKind::DropTemps(_)))
        .then_some(expr)
        .and_then(|expr| support::for_loop(cx, expr))
        .and_then(|loop_info| {
            support::block_only_expr(loop_info.body).map(|body| (loop_info, body))
        })
        .and_then(
            |(loop_info, body)| match support::peel_drop_temps(body).kind {
                ExprKind::If(condition, then_expr, None) => {
                    Some((loop_info, body, condition, then_expr))
                }
                _ => None,
            },
        )
        .and_then(
            |(loop_info, body, condition, then_expr)| match condition.kind {
                ExprKind::Let(let_expr)
                    if support::is_option(cx, cx.typeck_results().expr_ty(let_expr.init))
                        && single_action(then_expr) =>
                {
                    Some((loop_info, body))
                }
                _ => None,
            },
        )
        .and_then(|(loop_info, body)| {
            support::snippet(cx, body.span).map(|source| (loop_info, source))
        })
        .filter(|(_, source)| is_filter_map_source(source))
        .map(|(loop_info, _)| loop_info.span)
}

/// Return whether source syntax preserves the semantics of a filter-map closure.
fn is_filter_map_source(source: &str) -> bool {
    source.trim_start().starts_with("if let Some(")
        && !["?", ".await", "break", "continue", "return", ".ok()"]
            .iter()
            .any(|token| source.contains(token))
}

/// Returns whether an `if let` branch contains one terminal action.
fn single_action(expr: &Expr<'_>) -> bool {
    // Accept terminal expression categories that preserve immediate execution.
    let ExprKind::Block(block, _) = support::peel_drop_temps(expr).kind else {
        return false;
    };
    support::block_only_expr(block).is_some_and(|action| {
        matches!(
            support::peel_drop_temps(action).kind,
            ExprKind::Call(..) | ExprKind::MethodCall(..) | ExprKind::Assign(..)
        )
    })
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
