#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for manual `Option::take_if` expressions.
//!
//! It recognizes the standard `as_ref().is_some_and(...)` plus `take` conditional
//! and checks both receiver identity and the empty alternative before reporting.
//! The recommendation preserves the predicate while moving it to `Option::take_if`.
//! It avoids matching unrelated conditional expressions or nonstandard
//! option-like types.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_OPTION_TAKE_IF,
    Warn,
    "conditional `Option::take` can use `Option::take_if`",
    ManualOptionTakeIf
}

impl<'tcx> LateLintPass<'tcx> for ManualOptionTakeIf {
    /// Checks one `if` expression for a repeated standard `Option` receiver.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if take_if_candidate(cx, expr).is_some() {
            // Report the full conditional whose receiver can use `take_if`.
            support::emit(
                cx,
                MANUAL_OPTION_TAKE_IF,
                expr.span,
                "conditional `Option::take` can use `Option::take_if`",
                "use `Option::take_if`; adjust the predicate for its mutable reference",
            );
        }
    }
}

/// Return evidence for a conditional that can use `Option::take_if`.
fn take_if_candidate<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<()> {
    // Prove the result type, condition receiver, take receiver, and empty branch in order.
    let ExprKind::If(condition, then_expr, Some(else_expr)) = expr.kind else {
        return None;
    };
    support::is_option(cx, cx.typeck_results().expr_ty(expr))
        .then_some((condition, then_expr, else_expr))
        .and_then(|(condition, then_expr, else_expr)| {
            option_condition(cx, condition).and_then(|option_ref| {
                branch_expr(then_expr).map(|take_expr| (option_ref, take_expr, else_expr))
            })
        })
        .and_then(|(option_ref, take_expr, else_expr)| {
            take_call(cx, take_expr).map(|take_receiver| (option_ref, take_receiver, else_expr))
        })
        .filter(|(_, take_receiver, _)| {
            support::is_option(cx, cx.typeck_results().expr_ty(take_receiver))
        })
        .and_then(|(option_ref, take_receiver, else_expr)| {
            branch_expr(else_expr).and_then(|else_value| {
                support::snippet(cx, option_ref.span).and_then(|option_ref_source| {
                    support::snippet(cx, take_receiver.span).and_then(|take_source| {
                        support::snippet(cx, else_value.span)
                            .map(|else_source| (option_ref_source, take_source, else_source))
                    })
                })
            })
        })
        .filter(|(option_ref_source, take_source, else_source)| {
            option_ref_source == &format!("{take_source}.as_ref()") && else_source.trim() == "None"
        })
        .map(|_| ())
}

/// Return the borrowed receiver of `Option::is_some_and`.
fn option_condition<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(_, option_ref, [_], _) = expr.kind
        && support::method_path_ends_with(cx, expr, "::is_some_and")
    {
        Some(option_ref)
    } else {
        None
    }
}

/// Return the receiver of `Option::take`.
fn take_call<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::MethodCall(_, receiver, [], _) = expr.kind
        && support::method_path_ends_with(cx, expr, "::take")
    {
        Some(receiver)
    } else {
        None
    }
}

/// Returns the one expression inside a branch block.
fn branch_expr<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Block(block, _) = support::peel_drop_temps(expr).kind else {
        return Some(expr);
    };
    support::block_only_expr(block)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
