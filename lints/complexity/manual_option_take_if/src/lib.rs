#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for manual `Option::take_if` expressions.
//!
//! It recognizes `if place.as_ref().is_some_and(predicate) { place.take() }
//! else { None }`, resolves each call to the standard `Option` method, resolves
//! the `else` value to the `None` constructor, and requires both receivers to
//! name the same local place. The recommendation preserves the predicate while
//! moving it to `Option::take_if`, which tests and takes the value in one call.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    Expr, ExprKind, LangItem,
    def::{CtorOf, DefKind, Res},
};
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
        if is_take_if_candidate(cx, expr) {
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

/// Returns whether a conditional can use `Option::take_if`.
fn is_take_if_candidate<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    // Prove the condition receiver and the take receiver.
    let ExprKind::If(condition, then_expr, Some(else_expr)) = expr.kind else {
        return false;
    };
    let tested = option_method(cx, condition, "is_some_and")
        .and_then(|borrowed| option_method(cx, borrowed, "as_ref"));
    let taken = branch_expr(then_expr).and_then(|take| option_method(cx, take, "take"));

    // Require one place on both sides and an empty `else` branch.
    let is_empty_else = branch_expr(else_expr).is_some_and(|value| is_none(cx, value));
    tested
        .zip(taken)
        .is_some_and(|(tested, taken)| same_place(cx, tested, taken))
        && is_empty_else
}

/// Returns the receiver of a standard `Option` method with the given name.
fn option_method<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    name: &str,
) -> Option<&'tcx Expr<'tcx>> {
    // Resolve the method to an inherent `impl` block of `Option` itself.
    let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
        return None;
    };
    let method = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let owner = cx.tcx.inherent_impl_of_assoc(method)?;
    let is_option = cx
        .tcx
        .type_of(owner)
        .instantiate_identity()
        .skip_normalization()
        .ty_adt_def()
        .is_some_and(|adt| {
            cx.tcx
                .is_diagnostic_item(rustc_span::sym::Option, adt.did())
        });

    // Compare the resolved method name only after the owner is known.
    let is_named = cx.tcx.item_name(method).as_str() == name;
    (is_option && is_named).then_some(receiver)
}

/// Returns whether an expression is the standard `None` constructor.
fn is_none(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };
    matches!(
        cx.qpath_res(&qpath, expr.hir_id),
        Res::Def(DefKind::Ctor(CtorOf::Variant, _), ctor)
            if cx.tcx.is_lang_item(cx.tcx.parent(ctor), LangItem::OptionNone)
    )
}

/// Returns whether two receivers name the same local place.
fn same_place(cx: &LateContext<'_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    // Accept a local binding, then field projections and dereferences of the same place.
    match (left.kind, right.kind) {
        (ExprKind::Path(left_path), ExprKind::Path(right_path)) => matches!(
            (
                cx.qpath_res(&left_path, left.hir_id),
                cx.qpath_res(&right_path, right.hir_id),
            ),
            (Res::Local(left_id), Res::Local(right_id)) if left_id == right_id
        ),
        (ExprKind::Field(left_base, left_name), ExprKind::Field(right_base, right_name)) => {
            left_name.name == right_name.name && same_place(cx, left_base, right_base)
        }
        (ExprKind::Unary(left_op, left_base), ExprKind::Unary(right_op, right_base)) => {
            left_op == right_op && same_place(cx, left_base, right_base)
        }
        _ => false,
    }
}

/// Returns the one expression inside a branch block.
const fn branch_expr<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Block(block, _) = expr.kind else {
        return Some(expr);
    };
    support::block_only_expr(block)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
