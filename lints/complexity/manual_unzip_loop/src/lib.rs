#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

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

use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, Mutability, PatKind, Stmt, StmtKind, def::Res,
};
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
        let Some(tail) = block.expr else {
            return;
        };
        // Evaluate each adjacent pair of accumulators and its following loop.
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
            // Resolve the for-loop and recover its tuple-pattern source.
            let Some(loop_info) = support::for_loop(cx, loop_expr) else {
                continue;
            };
            let Some(loop_source) = support::snippet(cx, loop_info.span) else {
                continue;
            };
            let Some((first_item, second_item)) = pair_names(&loop_source) else {
                continue;
            };
            // Require tuple destructuring and the same ordered accumulators in the tail.
            let has_tuple_pattern = loop_source.trim_start().starts_with("for (");
            if !has_tuple_pattern {
                continue;
            }
            if !is_tuple(cx, tail, first, second) {
                continue;
            }
            // Verify that the body inserts each tuple element into its matching collection.
            if !is_unzip_body(cx, loop_info.body, first, second, first_item, second_item) {
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
    // Accept only standard collections constructed without initial contents.
    let source = support::snippet(cx, local.init?.span)?;
    (support::is_standard_collection(cx, cx.typeck_results().pat_ty(local.pat))
        && (source.ends_with("::new()") || source.ends_with("::default()")))
    .then_some(id)
}

/// Checks two direct insertions into the two ordered accumulators.
fn is_unzip_body<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    first: HirId,
    second: HirId,
    first_item: &str,
    second_item: &str,
) -> bool {
    // Compare ordered targets and direct tuple-field arguments.
    let [first_stmt, second_stmt] = body.stmts else {
        return false;
    };
    let (Some(first_action), Some(second_action)) = (
        support::stmt_expr(first_stmt),
        support::stmt_expr(second_stmt),
    ) else {
        return false;
    };
    insertion(cx, first_action) == Some((first, first_item.to_owned()))
        && insertion(cx, second_action) == Some((second, second_item.to_owned()))
}

/// Returns the local target and direct argument of one resolved insertion.
fn insertion<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<(HirId, String)> {
    // Resolve the insertion before returning its target and source argument.
    match expr.kind {
        ExprKind::MethodCall(_, receiver, [argument], _) => Some((receiver, argument)),
        _ => None,
    }
    .filter(|_| {
        support::method_path(cx, expr)
            .is_some_and(|path| path.ends_with("::push") || path.ends_with("::insert"))
    })
    .and_then(|(receiver, argument)| {
        binding_id(cx, receiver).zip(support::snippet(cx, argument.span))
    })
}

/// Extracts two simple binding names from the loop's tuple pattern.
fn pair_names(source: &str) -> Option<(&str, &str)> {
    let bindings = source.trim_start().strip_prefix("for (")?;
    let bindings = bindings.split_once(") in")?.0;
    let (first, second) = bindings.split_once(',')?;
    Some((first.trim(), second.trim()))
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
