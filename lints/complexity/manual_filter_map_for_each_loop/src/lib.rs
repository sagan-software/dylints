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

use rustc_hir::{Expr, ExprKind, LangItem};
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
        if let Some(span) = validate_filter_map_loop(cx, expr) {
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
fn validate_filter_map_loop<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    // Recover the loop, then require the resolved `Some` pattern and no early exit.
    (!matches!(expr.kind, ExprKind::DropTemps(_)))
        .then_some(expr)
        .and_then(|expr| support::for_loop(cx, expr))
        .filter(|loop_info| !loop_info.body.span.from_expansion())
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
        .filter(|(_, _, condition, then_expr)| {
            is_option_mapping(cx, condition) && single_action(then_expr)
        })
        .filter(|(_, body, _, _)| !support::contains_control_flow(body))
        .map(|(loop_info, _, _, _)| loop_info.span)
}

/// Returns whether a condition is `let Some(..) = option` on a mapped `Option`.
fn is_option_mapping(cx: &LateContext<'_>, condition: &Expr<'_>) -> bool {
    // Resolve the `Some` pattern rather than reading the source spelling.
    let ExprKind::Let(let_expr) = condition.kind else {
        return false;
    };
    let is_some_pattern = support::lang_ctor_pat(cx, let_expr.pat, LangItem::OptionSome).is_some();
    let is_option = support::is_option(cx, cx.typeck_results().expr_ty(let_expr.init));
    is_some_pattern && is_option && !is_result_ok(cx, let_expr.init)
}

/// Returns whether an expression calls `Result::ok`.
///
/// `if let Some(x) = result.ok()` is better written as `if let Ok(x) = result`,
/// so this lint leaves it to that rewrite.
fn is_result_ok(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    support::inherent_method(cx, expr)
        .is_some_and(|(self_ty, method)| method.as_str() == "ok" && support::is_result(cx, self_ty))
}

/// Returns whether an `if let` branch contains one terminal action.
fn single_action(expr: &Expr<'_>) -> bool {
    // Accept terminal expression categories that preserve immediate execution.
    matches!(
        support::peel_drop_temps(expr).kind,
        ExprKind::Block(block, _) if support::block_only_expr(block).is_some_and(|action| {
            matches!(
                support::peel_drop_temps(action).kind,
                ExprKind::Call(..) | ExprKind::MethodCall(..) | ExprKind::Assign(..)
            )
        })
    )
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
