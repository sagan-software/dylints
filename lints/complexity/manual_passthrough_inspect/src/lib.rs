#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unsupported rustc statement and pattern variants stay outside this source shape"
)]
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

use rustc_hir::{Block, Expr, ExprKind, HirId, PatKind, StmtKind, def::Res};
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
    // Require the complete three-part shape before inspecting variant source.
    match (block.stmts, block.expr) {
        ([binding_stmt, observation_stmt], Some(tail)) => {
            Some((binding_stmt, observation_stmt, tail))
        }
        _ => None,
    }
    .and_then(
        |(binding_stmt, observation_stmt, tail)| match binding_stmt.kind {
            StmtKind::Let(local) => Some((binding_stmt, observation_stmt, tail, local)),
            _ => None,
        },
    )
    // Require a simple binding whose identity can be followed through the block.
    .and_then(
        |(binding_stmt, observation_stmt, tail, local)| match local.pat.kind {
            PatKind::Binding(_, id, ident, None) => {
                Some((binding_stmt, observation_stmt, tail, local, id, ident))
            }
            _ => None,
        },
    )
    .filter(|(_, _, tail, local, id, _)| {
        local.init.is_some()
            && (support::is_option(cx, cx.typeck_results().pat_ty(local.pat))
                || support::is_result(cx, cx.typeck_results().pat_ty(local.pat)))
            && binding_id(cx, tail) == Some(*id)
    })
    .and_then(|(binding_stmt, observation_stmt, _, _, _, ident)| {
        support::stmt_expr(observation_stmt).and_then(|observation| {
            support::snippet(cx, observation.span)
                .map(|source| (binding_stmt, observation, source, ident))
        })
    })
    .filter(|(_, observation, source, ident)| {
        let name = ident.name;
        let borrowed_binding = format!("&{name}");
        let is_supported_variant = source.contains("if let Some(")
            || source.contains("if let Ok(")
            || source.contains("if let Err(");
        is_supported_variant
            && source.contains(&borrowed_binding)
            && single_observation(cx, observation)
    })
    .map(|(binding_stmt, observation, _, _)| binding_stmt.span.to(observation.span))
}

/// Returns whether the observation branch has one direct action.
fn single_observation(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Exclude control flow and multi-statement observation bodies.
    let Some(source) = support::snippet(cx, expr.span) else {
        return false;
    };
    source.matches(';').count() <= 1
        && !["?", ".await", "break", "continue", "return"]
            .iter()
            .any(|token| source.contains(token))
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
