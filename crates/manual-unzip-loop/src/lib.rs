#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for loops that can use `Iterator::unzip`.
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

use rustc_hir::{Block, Expr, ExprKind, HirId, Pat, PatKind, Stmt, StmtKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_UNZIP_LOOP,
    Warn,
    "pair collection loop can use `Iterator::unzip`",
    ManualUnzipLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualUnzipLoop {
    /// Checks two new collections, a pair loop, and the matching tuple tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Scan only adjacent initializer pairs followed by one loop.
        if block.span.from_expansion() {
            return;
        }
        let Some(tail) = block.expr else {
            return;
        };
        // Evaluate each adjacent pair of accumulators and its following loop.
        for [first_stmt, second_stmt, loop_stmt] in block.stmts.array_windows() {
            let (Some(first), Some(second), Some(loop_info)) = (
                accumulator(cx, first_stmt),
                accumulator(cx, second_stmt),
                support::stmt_expr(loop_stmt).and_then(|expr| support::for_loop(cx, expr)),
            ) else {
                continue;
            };
            // Require a two-binding tuple pattern and the same ordered accumulators in the tail.
            let Some((first_item, second_item)) = pair_bindings(loop_info.pat) else {
                continue;
            };
            if !is_tuple(cx, tail, (first, second))
                || !is_unzip_body(
                    cx,
                    loop_info.body,
                    ((first, first_item), (second, second_item)),
                )
            {
                continue;
            }
            support::emit(
                cx,
                MANUAL_UNZIP_LOOP,
                loop_info.span,
                "pair collection loop can use `Iterator::unzip`",
                "use `Iterator::unzip` and bind its two returned collections",
            );
        }
    }
}

/// Returns the binding of an empty mutable standard collection.
fn accumulator(cx: &LateContext<'_>, stmt: &Stmt<'_>) -> Option<HirId> {
    // Accept only standard collections constructed without initial contents.
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

/// Returns the two bindings of a `(first, second)` loop pattern.
fn pair_bindings(pat: &Pat<'_>) -> Option<(HirId, HirId)> {
    // A rest pattern would skip fields that `unzip` cannot drop.
    let PatKind::Tuple([first, second], dot_dot) = pat.kind else {
        return None;
    };
    if dot_dot.as_opt_usize().is_some() {
        return None;
    }
    support::simple_binding(first).zip(support::simple_binding(second))
}

/// Checks two direct insertions of the tuple bindings into their ordered accumulators.
fn is_unzip_body<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    (first_expected, second_expected): ((HirId, HirId), (HirId, HirId)),
) -> bool {
    // Accept the second insertion as a statement or as the block tail.
    let actions = match (body.stmts, body.expr) {
        ([first, second], None) => support::stmt_expr(first).zip(support::stmt_expr(second)),
        ([first], Some(second)) => support::stmt_expr(first).map(|first| (first, second)),
        _ => None,
    };
    actions.is_some_and(|(first, second)| {
        insertion(cx, first) == Some(first_expected)
            && insertion(cx, second) == Some(second_expected)
    })
}

/// Returns the local target and local argument of one resolved collection insertion.
fn insertion(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<(HirId, HirId)> {
    // Resolve the insertion before returning its target and argument bindings.
    let ExprKind::MethodCall(_, receiver, [argument], _) = expr.kind else {
        return None;
    };
    let is_insertion = support::standard_collection_method(cx, expr)
        .is_some_and(|(_, method)| matches!(method.as_str(), "push" | "push_back" | "insert"));
    is_insertion
        .then(|| support::local_binding(cx, receiver).zip(support::local_binding(cx, argument)))
        .flatten()
}

/// Checks the ordered tuple tail.
fn is_tuple(cx: &LateContext<'_>, expr: &Expr<'_>, (first, second): (HirId, HirId)) -> bool {
    let ExprKind::Tup([left, right]) = support::peel_drop_temps(expr).kind else {
        return false;
    };
    support::local_binding(cx, left) == Some(first)
        && support::local_binding(cx, right) == Some(second)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
