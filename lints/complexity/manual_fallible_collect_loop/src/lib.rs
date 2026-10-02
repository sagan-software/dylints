#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for loops that can use fallible `Iterator::collect`.
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

use rustc_hir::{Block, Expr, ExprKind, HirId, LangItem, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_FALLIBLE_COLLECT_LOOP,
    Warn,
    "fallible collection loop can use `Iterator::collect`",
    ManualFallibleCollectLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualFallibleCollectLoop {
    /// Checks adjacent accumulator and loop statements with a success tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Establish the fallible success value before scanning local statement pairs.
        if block.span.from_expansion() {
            return;
        }
        let Some((residual, value)) = block.expr.and_then(|tail| success_value(cx, tail)) else {
            return;
        };
        // Evaluate each accumulator and its immediately following loop.
        for [accumulator_stmt, loop_stmt] in block.stmts.array_windows() {
            let Some(accumulator) = accumulator(cx, accumulator_stmt) else {
                continue;
            };
            // Resolve the loop only after establishing its accumulator identity.
            let Some(loop_info) =
                support::stmt_expr(loop_stmt).and_then(|expr| support::for_loop(cx, expr))
            else {
                continue;
            };
            // Require the same accumulator in both the insertion and success tail.
            let is_tail_match = support::local_binding(cx, value) == Some(accumulator);
            if !is_tail_match || !is_fallible_insertion(cx, loop_info.body, accumulator, residual) {
                continue;
            }
            support::emit(
                cx,
                MANUAL_FALLIBLE_COLLECT_LOOP,
                loop_info.span,
                "fallible collection loop can use `Iterator::collect`",
                "use `map(...).collect()` with the matching fallible collection type",
            );
        }
    }
}

/// A collection residual accepted by standard `collect`.
#[derive(Clone, Copy)]
enum Residual<'tcx> {
    /// Standard `Result` with this error type.
    Result(Ty<'tcx>),
    /// Standard `Option`.
    Option,
}

/// Returns the residual and value of an `Ok(value)` or `Some(value)` tail.
fn success_value<'tcx>(
    cx: &LateContext<'tcx>,
    tail: &'tcx Expr<'tcx>,
) -> Option<(Residual<'tcx>, &'tcx Expr<'tcx>)> {
    // Resolve the constructor and retain the residual required by `collect`.
    let ty::Adt(_, args) = cx.typeck_results().expr_ty(tail).kind() else {
        return None;
    };
    support::lang_ctor_call(cx, tail, LangItem::ResultOk)
        .map(|value| (Residual::Result(args.type_at(1)), value))
        .or_else(|| {
            support::lang_ctor_call(cx, tail, LangItem::OptionSome)
                .map(|value| (Residual::Option, value))
        })
}

/// Returns the binding of an empty mutable standard collection.
fn accumulator(cx: &LateContext<'_>, stmt: &Stmt<'_>) -> Option<HirId> {
    // Require a standard collection constructed by an argument-free `new` or `default`.
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };
    let id = support::simple_binding(local.pat)?;
    (support::is_standard_collection(cx, cx.typeck_results().pat_ty(local.pat))
        && local
            .init
            .is_some_and(|init| support::is_empty_constructor(cx, init)))
    .then_some(id)
}

/// Checks one standard collection insertion whose argument has one matching `?`.
fn is_fallible_insertion<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    id: HirId,
    residual: Residual<'tcx>,
) -> bool {
    // Match one resolved insertion into the accumulator.
    let Some(action) = support::block_only_expr(body) else {
        return false;
    };
    let ExprKind::MethodCall(_, receiver, [argument], _) = action.kind else {
        return false;
    };
    let is_insertion = support::standard_collection_method(cx, action)
        .is_some_and(|(_, method)| matches!(method.as_str(), "push" | "push_back" | "insert"));
    if !is_insertion || support::local_binding(cx, receiver) != Some(id) {
        return false;
    }
    // Require exactly one `?` whose operand has the tail's residual, and no other exit.
    let exits = support::early_exits(argument);
    match exits.try_operands.as_slice() {
        [operand] if !exits.has_other => {
            is_residual_match(cx, cx.typeck_results().expr_ty(operand), residual)
        }
        _ => false,
    }
}

/// Returns whether a type has the required standard collection residual.
fn is_residual_match<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, residual: Residual<'tcx>) -> bool {
    // Compare the residual while allowing the collected success type to differ.
    match residual {
        Residual::Result(error_ty) => {
            support::is_result(cx, ty)
                && matches!(ty.kind(), ty::Adt(_, args) if args.type_at(1) == error_ty)
        }
        Residual::Option => support::is_option(cx, ty),
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
