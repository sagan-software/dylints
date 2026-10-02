#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for loops that can use `Extend::extend`.
//!
//! It recognizes a standard `for` loop whose body pushes each source item into
//! a different `Vec` or `VecDeque`. Resolved method paths, source comparison,
//! and control-flow screening keep the recommendation limited to loops that
//! have a direct `Extend::extend` equivalent.

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
    pub MANUAL_EXTEND_LOOP,
    Warn,
    "collection insertion loop can use `Extend::extend`",
    ManualExtendLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualExtendLoop {
    /// Checks a standard loop for one resolved sequence insertion.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(loop_span) = extend_loop_span(cx, expr) {
            // Report the verified loop span with its equivalent Extend operation.
            support::emit(
                cx,
                MANUAL_EXTEND_LOOP,
                loop_span,
                "collection insertion loop can use `Extend::extend`",
                "use `Extend::extend`, with `map` when the inserted value is transformed",
            );
        }
    }
}

/// Returns a loop span when its body is a direct standard sequence insertion.
fn extend_loop_span<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<rustc_span::Span> {
    (!matches!(expr.kind, ExprKind::DropTemps(_)))
        .then_some(expr)
        .and_then(|expr| support::for_loop(cx, expr))
        .and_then(|loop_info| {
            support::block_only_expr(loop_info.body).map(|action| (loop_info, action))
        })
        .and_then(|(loop_info, action)| {
            standard_insertion_receiver(cx, action).map(|receiver| (loop_info, action, receiver))
        })
        .filter(|(loop_info, action, receiver)| {
            is_valid_extend_body(cx, loop_info, action, receiver)
        })
        .map(|(loop_info, _, _)| loop_info.span)
}

/// Returns the receiver for one resolved `Vec` or `VecDeque` insertion call.
fn standard_insertion_receiver<'tcx>(
    cx: &LateContext<'tcx>,
    action: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Remove compiler-generated wrappers before resolving the user-written call.
    let action = support::peel_drop_temps(action);
    let ExprKind::MethodCall(_, receiver, [_], _) = action.kind else {
        return None;
    };
    // Use the resolved method path so similarly named local methods are excluded.
    let path = support::method_path(cx, action)?;
    let is_standard_insertion = (path.contains("::vec::Vec") && path.ends_with("::push"))
        || (path.contains("::collections::VecDeque") && path.ends_with("::push_back"));
    is_standard_insertion
        .then_some(receiver)
        .filter(|receiver| support::is_standard_sequence(cx, cx.typeck_results().expr_ty(receiver)))
}

/// Checks source identity and control-flow restrictions for one loop body.
fn is_valid_extend_body<'tcx>(
    cx: &LateContext<'tcx>,
    loop_info: &support::ForLoop<'tcx>,
    action: &'tcx Expr<'tcx>,
    receiver: &'tcx Expr<'tcx>,
) -> bool {
    let (Some(source), Some(target), Some(body)) = (
        support::snippet(cx, loop_info.source.span),
        support::snippet(cx, receiver.span),
        support::snippet(cx, action.span),
    ) else {
        return false;
    };
    let has_control_flow = ["?", ".await", "break", "continue", "return"]
        .iter()
        .any(|token| body.contains(token));
    source != target && !has_control_flow
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
