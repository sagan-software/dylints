#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

//! A lint for loops that can use `filter(...).for_each(...)`.
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
    pub MANUAL_FILTER_FOR_EACH_LOOP,
    Warn,
    "filtered `for` loop can use `filter(...).for_each(...)`",
    ManualFilterForEachLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualFilterForEachLoop {
    /// Checks one standard loop for a predicate-only branch.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) {
            return;
        }
        if let Some(span) = filtered_loop_span(cx, expr) {
            // Report the verified loop span with its equivalent adapter chain.
            support::emit(
                cx,
                MANUAL_FILTER_FOR_EACH_LOOP,
                span,
                "filtered `for` loop can use `filter(...).for_each(...)`",
                "use `filter(...).for_each(...)` for this filtered action",
            );
        }
    }
}

/// Return the span of a loop whose body is one safe filtered action.
fn filtered_loop_span<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    // Keep the candidate pipeline ordered from compiler shape to source safety.
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
        .filter(|(_, _, condition, _)| cx.typeck_results().expr_ty(condition).is_bool())
        .filter(|(_, _, _, then_expr)| single_action(then_expr))
        .and_then(|(loop_info, body, _, _)| {
            support::snippet(cx, body.span).map(|source| (loop_info, source))
        })
        .filter(|(_, source)| {
            !["?", ".await", "break", "continue", "return"]
                .iter()
                .any(|token| source.contains(token))
        })
        .map(|(loop_info, _)| loop_info.span)
}

/// Returns whether an `if` branch contains one terminal action.
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
