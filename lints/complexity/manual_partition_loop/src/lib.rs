#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unsupported rustc expression variants do not form direct insertion branches"
)]
#![warn(unused_extern_crates)]

//! A lint for loops that can use `Iterator::partition`.
//!
//! It recognizes two empty mutable standard collections, a loop with opposite
//! insertion branches, and a tuple containing the collections afterward. The
//! check requires resolved bindings and matching source expressions so a
//! superficially similar loop with different ownership or branch behavior is
//! not rewritten by an automated tool.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, Mutability, PatKind, Stmt, StmtKind, def::Res,
};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_PARTITION_LOOP,
    Warn,
    "two-way collection loop can use `Iterator::partition`",
    ManualPartitionLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualPartitionLoop {
    /// Checks two new collections, one loop, and the matching tuple tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Scan only adjacent initializer pairs followed by one loop.
        let Some(tail) = block.expr else {
            return;
        };

        // Resolve each adjacent pair of accumulators and its following loop.
        for statements in block.stmts.windows(3) {
            let [first_stmt, second_stmt, loop_stmt] = statements else {
                continue;
            };
            let (Some(first), Some(second), Some(loop_expr)) = (
                accumulator(cx, first_stmt),
                accumulator(cx, second_stmt),
                support::stmt_expr(loop_stmt),
            ) else {
                continue;
            };
            let Some(loop_info) = support::for_loop(cx, loop_expr) else {
                continue;
            };
            if !is_tuple(cx, tail, first, second) {
                continue;
            }
            if !is_partition_body(cx, loop_info.body, first, second) {
                continue;
            }

            // Report the loop after its initializers, branches, and tuple tail agree.
            support::emit(
                cx,
                MANUAL_PARTITION_LOOP,
                loop_info.span,
                "two-way collection loop can use `Iterator::partition`",
                "use `Iterator::partition` and bind its two returned collections",
            );
        }
    }
}

/// Parses an empty mutable standard collection binding.
fn accumulator(cx: &LateContext<'_>, stmt: &Stmt<'_>) -> Option<HirId> {
    // Require a mutable empty standard collection with one local identity.
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Mut), id, _, None) = local.pat.kind
    else {
        return None;
    };

    // Accept only empty constructors for resolved standard collection types.
    let source = support::snippet(cx, local.init?.span)?;
    (support::is_standard_collection(cx, cx.typeck_results().pat_ty(local.pat))
        && (source.ends_with("::new()") || source.ends_with("::default()")))
    .then_some(id)
}

/// Checks opposite branches that insert the same source expression.
fn is_partition_body<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    first: HirId,
    second: HirId,
) -> bool {
    // Match opposite direct insertion branches after proving a boolean predicate.
    let Some(body) = support::block_only_expr(block) else {
        return false;
    };
    let ExprKind::If(condition, true_branch, Some(false_branch)) = body.kind else {
        return false;
    };
    if !cx.typeck_results().expr_ty(condition).is_bool() {
        return false;
    }

    // Resolve both branches to direct insertions into the two accumulators.
    let (Some((true_id, true_arg)), Some((false_id, false_arg))) =
        (branch_push(cx, true_branch), branch_push(cx, false_branch))
    else {
        return false;
    };
    true_id == first
        && false_id == second
        && support::snippet(cx, true_arg.span) == support::snippet(cx, false_arg.span)
}

/// Returns the target binding and argument from one direct standard insertion.
fn branch_push<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(HirId, &'tcx Expr<'tcx>)> {
    // Resolve the insertion method before returning its target and argument.
    match expr.kind {
        ExprKind::Block(block, _) => Some(block),
        _ => None,
    }
    .and_then(support::block_only_expr)
    .and_then(|action| match action.kind {
        ExprKind::MethodCall(_, receiver, [argument], _) => Some((action, receiver, argument)),
        _ => None,
    })
    .and_then(|(action, receiver, argument)| {
        support::method_path(cx, action)
            .filter(|path| path.ends_with("::push") || path.ends_with("::insert"))
            .map(|_| (receiver, argument))
    })
    .and_then(|(receiver, argument)| binding_id(cx, receiver).map(|id| (id, argument)))
}

/// Checks the ordered tuple tail.
fn is_tuple(cx: &LateContext<'_>, expr: &Expr<'_>, first: HirId, second: HirId) -> bool {
    let ExprKind::Tup([left, right]) = support::peel_drop_temps(expr).kind else {
        return false;
    };
    binding_id(cx, left) == Some(first) && binding_id(cx, right) == Some(second)
}

/// Returns the local binding resolved by a path expression.
fn binding_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    // Use HIR resolution so shadowed names remain distinct.
    let ExprKind::Path(qpath) = support::peel_drop_temps(expr).kind else {
        return None;
    };
    let Res::Local(id) = cx.qpath_res(&qpath, expr.hir_id) else {
        return None;
    };
    Some(id)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
