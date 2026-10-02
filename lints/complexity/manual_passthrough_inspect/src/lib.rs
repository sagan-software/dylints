#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint for manual pass-through inspection of `Option` and `Result`.
//!
//! It recognizes a binding, one direct variant observation, and an unchanged
//! returned value whose side effect can move onto `inspect` or `inspect_err`.
//! The check requires resolved binding identity, a supported standard type, and
//! a small observation body before it emits its structural recommendation.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    Block, BorrowKind, Expr, ExprKind, HirId, LangItem, LetStmt, Mutability, Pat, StmtKind,
};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_PASSTHROUGH_INSPECT,
    Warn,
    "unchanged `Option` or `Result` observation can use an inspect method",
    ManualPassthroughInspect
}

impl<'tcx> LateLintPass<'tcx> for ManualPassthroughInspect {
    /// Checks an initializer, adjacent observation, and unchanged block tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        if let Some(span) = passthrough_span(cx, block) {
            support::emit(
                cx,
                MANUAL_PASSTHROUGH_INSPECT,
                span,
                "unchanged `Option` or `Result` observation can use an inspect method",
                "use `inspect` or `inspect_err` on the initializer and return the chain",
            );
        }
    }
}

/// Returns the source span of one safe pass-through observation.
fn passthrough_span(cx: &LateContext<'_>, block: &Block<'_>) -> Option<rustc_span::Span> {
    // Require the complete three-part shape outside macro-generated code.
    let ([binding_stmt, observation_stmt], Some(tail)) = (block.stmts, block.expr) else {
        return None;
    };
    let StmtKind::Let(local) = binding_stmt.kind else {
        return None;
    };
    // Follow the binding identity from the `let` through the observation to the tail.
    let id = returned_binding(cx, local, tail).filter(|_| !block.span.from_expansion())?;
    let observation = support::stmt_expr(observation_stmt)?;
    is_observation(cx, observation, id).then(|| binding_stmt.span.to(observation.span))
}

/// Returns the binding of an owned `Option` or `Result` that the tail returns unchanged.
fn returned_binding(cx: &LateContext<'_>, local: &LetStmt<'_>, tail: &Expr<'_>) -> Option<HirId> {
    // A borrowed binding cannot move into `inspect`, and `let else` adds a branch.
    let id = support::simple_binding(local.pat)?;
    let ty = cx.typeck_results().pat_ty(local.pat);
    let is_supported_type =
        !ty.is_ref() && (support::is_option(cx, ty) || support::is_result(cx, ty));
    // The tail must return the same binding, not a shadowing one.
    let is_plain_let = local.init.is_some() && local.els.is_none();
    let is_returned = support::local_binding(cx, tail) == Some(id);
    (is_supported_type && is_plain_let && is_returned).then_some(id)
}

/// Returns whether an expression is `if let Variant(..) = &binding { action }`.
fn is_observation(cx: &LateContext<'_>, expr: &Expr<'_>, id: HirId) -> bool {
    // Match a borrowed variant test without `else`.
    let ExprKind::If(condition, then_expr, None) = expr.kind else {
        return false;
    };
    let ExprKind::Let(let_expr) = condition.kind else {
        return false;
    };
    let ExprKind::AddrOf(BorrowKind::Ref, Mutability::Not, borrowed) = let_expr.init.kind else {
        return false;
    };
    // Resolve both the borrowed binding and the variant constructor.
    let is_binding = support::local_binding(cx, borrowed) == Some(id);
    let is_variant = is_variant_pattern(cx, let_expr.pat);
    // Allow one direct action and no exit from the enclosing function or loop.
    let is_single_action = matches!(
        then_expr.kind,
        ExprKind::Block(block, _) if support::block_only_expr(block).is_some()
    );
    let has_exit = support::contains_control_flow(expr);
    is_binding && is_variant && is_single_action && !has_exit
}

/// Returns whether a pattern is `Some(..)`, `Ok(..)`, or `Err(..)`.
fn is_variant_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    [
        LangItem::OptionSome,
        LangItem::ResultOk,
        LangItem::ResultErr,
    ]
    .into_iter()
    .any(|item| support::lang_ctor_pat(cx, pat, item).is_some_and(is_exhaustive_payload))
}

/// Returns whether a variant payload pattern accepts every payload value.
const fn is_exhaustive_payload(pat: &Pat<'_>) -> bool {
    matches!(
        pat.kind,
        rustc_hir::PatKind::Wild | rustc_hir::PatKind::Binding(_, _, _, None)
    )
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
