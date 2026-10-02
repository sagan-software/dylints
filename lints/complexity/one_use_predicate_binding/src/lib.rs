#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint traverses rustc syntax comprehensively and intentionally ignores diagnostic builders and unrelated variants"
)]
#![expect(
    clippy::too_many_lines,
    reason = "the syntax traversal keeps every use-counting branch in one function"
)]

//! A lint to check for one-use predicate bindings.
//!
//! It identifies a local binding that stores one predicate and is consumed once
//! in a nearby conditional, then recommends inlining the expression when that
//! rewrite preserves evaluation order. The traversal follows resolved uses and
//! rejects shadowing, mutation, control-flow, and syntax it cannot prove safe.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    BinOpKind, BindingMode, Block, ByRef, Expr, ExprKind, HirId, Mutability, Pat, PatKind, QPath,
    Stmt, StmtKind, UnOp, def::Res,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub ONE_USE_PREDICATE_BINDING,
    Warn,
    "one-use predicate binding could be inlined into the branch condition",
    OneUsePredicateBinding
}

impl<'tcx> LateLintPass<'tcx> for OneUsePredicateBinding {
    /// Check block for this lint.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Keep the lint local so the suggestion cannot cross intervening side effects.
        for index in 0..block.stmts.len() {
            let Some(offense) = one_use_predicate_binding(cx, block, index) else {
                continue;
            };
            emit_span_lint_with_help(
                cx,
                ONE_USE_PREDICATE_BINDING,
                offense.span,
                "one-use predicate binding could be inlined",
                "inline the predicate expression into the branch condition unless the name captures a domain concept",
            );
        }
    }
}

/// State used by the predicate binding analysis.
struct PredicateBinding {
    /// span stored for this lint's analysis.
    span: Span,
}

/// Helper for one use predicate binding analysis.
fn one_use_predicate_binding<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    index: usize,
) -> Option<PredicateBinding> {
    let local = predicate_local(cx, block.stmts.get(index)?)?;
    let branch = adjacent_branch(block, index)?;

    if !condition_is_binding(cx, branch.condition, local.hir_id)
        || !branch.allows_inlining
        || !is_bool_expr(cx, local.init)
        || !is_predicate_initializer(local.init)
        || !is_generic_predicate_name(local.name)
    {
        return None;
    }

    // Count resolved uses in the whole tail so body uses, later branches, and assignments suppress
    // a warning even when another local happens to share the same name elsewhere.
    (count_local_uses_in_tail(cx, block, index + 1, local.hir_id) == 1)
        .then_some(PredicateBinding { span: local.span })
}

/// State used by the predicate local analysis.
struct PredicateLocal<'tcx> {
    /// name stored for this lint's analysis.
    name: Symbol,
    /// hir id stored for this lint's analysis.
    hir_id: HirId,
    /// span stored for this lint's analysis.
    span: Span,
    /// init stored for this lint's analysis.
    init: &'tcx Expr<'tcx>,
}

/// Helper for predicate local analysis.
fn predicate_local<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
) -> Option<PredicateLocal<'tcx>> {
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };

    let (name, hir_id) = immutable_ident(local.pat)?;
    let init = local.init?;

    // Prefer the typed local pattern when available; this protects annotated non-bool lookalikes.
    if !matches!(cx.typeck_results().pat_ty(local.pat).kind(), ty::Bool) {
        return None;
    }

    // Retain the binding identity and initializer for the later use-count analysis.
    Some(PredicateLocal {
        name,
        hir_id,
        span: local.span,
        init,
    })
}

/// Helper for immutable ident analysis.
const fn immutable_ident(pat: &Pat<'_>) -> Option<(Symbol, HirId)> {
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Not), hir_id, ident, None) = pat.kind
    else {
        return None;
    };

    Some((ident.name, hir_id))
}

/// State used by the branch condition analysis.
struct BranchCondition<'tcx> {
    /// condition stored for this lint's analysis.
    condition: &'tcx Expr<'tcx>,
    /// allows inlining stored for this lint's analysis.
    allows_inlining: bool,
}

/// Helper for adjacent branch analysis.
fn adjacent_branch<'tcx>(
    block: &'tcx Block<'tcx>,
    local_index: usize,
) -> Option<BranchCondition<'tcx>> {
    if let Some(next_stmt) = block.stmts.get(local_index + 1) {
        return branch_condition(stmt_expr(next_stmt)?);
    }

    branch_condition(block.expr?)
}

/// Helper for branch condition analysis.
const fn branch_condition<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<BranchCondition<'tcx>> {
    match expr.kind {
        ExprKind::If(condition, _, _) => Some(BranchCondition {
            condition,
            allows_inlining: true,
        }),
        _ => None,
    }
}

/// Helper for condition is binding analysis.
fn condition_is_binding(cx: &LateContext<'_>, condition: &Expr<'_>, hir_id: HirId) -> bool {
    path_is_binding(cx, condition, hir_id)
        || matches!(
            condition.kind,
            ExprKind::Unary(UnOp::Not, inner) if path_is_binding(cx, inner, hir_id)
        )
}

/// Return whether bool expr.
fn is_bool_expr(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(cx.typeck_results().expr_ty(expr).kind(), ty::Bool)
}

/// Return whether predicate initializer.
fn is_predicate_initializer(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Binary(op, lhs, rhs) => match op.node {
            BinOpKind::Eq
            | BinOpKind::Lt
            | BinOpKind::Le
            | BinOpKind::Ne
            | BinOpKind::Ge
            | BinOpKind::Gt => true,
            BinOpKind::And | BinOpKind::Or => {
                is_predicate_initializer(lhs) || is_predicate_initializer(rhs)
            }
            _ => false,
        },
        ExprKind::MethodCall(segment, ..) => is_predicate_function_name(segment.ident.name),
        ExprKind::Call(callee, _) => {
            path_last_segment_name(callee).is_some_and(is_predicate_function_name)
        }
        ExprKind::Unary(UnOp::Not, inner) => is_predicate_initializer(inner),
        _ => false,
    }
}

/// Return whether predicate function name.
fn is_predicate_function_name(name: Symbol) -> bool {
    let name = name.as_str();

    name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("should_")
        || matches!(
            name,
            "contains"
                | "matches"
                | "starts_with"
                | "ends_with"
                | "eq_ignore_ascii_case"
                | "is_ascii"
        )
}

/// Return whether generic predicate name.
fn is_generic_predicate_name(name: Symbol) -> bool {
    // Split the identifier into vocabulary tokens before checking its predicate prefix.
    let name = name.as_str();
    let words = name
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let Some(tail) = predicate_tail(&words) else {
        return false;
    };

    !tail.is_empty() && tail.iter().all(|word| is_generic_predicate_word(word))
}

/// Helper for predicate tail analysis.
fn predicate_tail<'a>(words: &'a [&str]) -> Option<&'a [&'a str]> {
    match words {
        [
            "is" | "has" | "have" | "can" | "should" | "contains" | "matches",
            tail @ ..,
        ] => Some(tail),
        _ => None,
    }
}

/// Return whether generic predicate word.
fn is_generic_predicate_word(word: &str) -> bool {
    matches!(
        word,
        "empty"
            | "not"
            | "non"
            | "some"
            | "none"
            | "ok"
            | "err"
            | "valid"
            | "invalid"
            | "ready"
            | "present"
            | "missing"
            | "found"
            | "available"
            | "enabled"
            | "disabled"
            | "positive"
            | "negative"
            | "zero"
            | "item"
            | "items"
            | "value"
            | "values"
            | "result"
            | "results"
            | "data"
            | "match"
            | "matches"
            | "remaining"
    )
}

/// Count local uses in tail used by the lint.
fn count_local_uses_in_tail<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    tail_start: usize,
    hir_id: HirId,
) -> usize {
    block
        .stmts
        .get(tail_start..)
        .unwrap_or_default()
        .iter()
        .map(|stmt| count_local_uses_in_stmt(cx, stmt, hir_id))
        .sum::<usize>()
        + block
            .expr
            .map_or(0, |expr| count_local_uses_in_expr(cx, expr, hir_id))
}

/// Count local uses in stmt used by the lint.
fn count_local_uses_in_stmt(cx: &LateContext<'_>, stmt: &Stmt<'_>, hir_id: HirId) -> usize {
    match stmt.kind {
        StmtKind::Let(local) => local
            .init
            .map_or(0, |init| count_local_uses_in_expr(cx, init, hir_id)),
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => count_local_uses_in_expr(cx, expr, hir_id),
        StmtKind::Item(_) => 0,
    }
}

/// Count local uses in expr used by the lint.
fn count_local_uses_in_expr(cx: &LateContext<'_>, expr: &Expr<'_>, hir_id: HirId) -> usize {
    // Count a resolved reference to the selected binding as one use.
    if path_is_binding(cx, expr, hir_id) {
        return 1;
    }

    // Recurse through expression shapes that can contain executable child expressions.
    match expr.kind {
        ExprKind::Call(callee, args) => {
            count_local_uses_in_expr(cx, callee, hir_id)
                + args
                    .iter()
                    .map(|arg| count_local_uses_in_expr(cx, arg, hir_id))
                    .sum::<usize>()
        }
        ExprKind::MethodCall(_, receiver, args, _) => {
            count_local_uses_in_expr(cx, receiver, hir_id)
                + args
                    .iter()
                    .map(|arg| count_local_uses_in_expr(cx, arg, hir_id))
                    .sum::<usize>()
        }
        ExprKind::Binary(_, lhs, rhs)
        | ExprKind::Assign(lhs, rhs, _)
        | ExprKind::AssignOp(_, lhs, rhs)
        | ExprKind::Index(lhs, rhs, _) => {
            count_local_uses_in_expr(cx, lhs, hir_id) + count_local_uses_in_expr(cx, rhs, hir_id)
        }
        ExprKind::Unary(_, inner)
        | ExprKind::Use(inner, _)
        | ExprKind::Cast(inner, _)
        | ExprKind::Type(inner, _)
        | ExprKind::DropTemps(inner)
        | ExprKind::Field(inner, _)
        | ExprKind::AddrOf(_, _, inner)
        | ExprKind::Ret(Some(inner))
        | ExprKind::Break(_, Some(inner)) => count_local_uses_in_expr(cx, inner, hir_id),
        ExprKind::Block(block, _) | ExprKind::Loop(block, ..) => {
            count_local_uses_in_block(cx, block, hir_id)
        }
        ExprKind::If(condition, then_expr, else_expr) => {
            count_local_uses_in_expr(cx, condition, hir_id)
                + count_local_uses_in_expr(cx, then_expr, hir_id)
                + else_expr.map_or(0, |else_expr| {
                    count_local_uses_in_expr(cx, else_expr, hir_id)
                })
        }
        ExprKind::Match(scrutinee, arms, _) => {
            count_local_uses_in_expr(cx, scrutinee, hir_id)
                + arms
                    .iter()
                    .map(|arm| {
                        arm.guard
                            .map_or(0, |guard| count_local_uses_in_expr(cx, guard, hir_id))
                            + count_local_uses_in_expr(cx, arm.body, hir_id)
                    })
                    .sum::<usize>()
        }
        _ => 0,
    }
}

/// Count local uses in block used by the lint.
fn count_local_uses_in_block(cx: &LateContext<'_>, block: &Block<'_>, hir_id: HirId) -> usize {
    block
        .stmts
        .iter()
        .map(|stmt| count_local_uses_in_stmt(cx, stmt, hir_id))
        .sum::<usize>()
        + block
            .expr
            .map_or(0, |expr| count_local_uses_in_expr(cx, expr, hir_id))
}

/// Helper for stmt expr analysis.
const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Helper for path is binding analysis.
fn path_is_binding(cx: &LateContext<'_>, expr: &Expr<'_>, hir_id: HirId) -> bool {
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };

    matches!(
        cx.typeck_results().qpath_res(&qpath, expr.hir_id),
        Res::Local(resolved_hir_id) if resolved_hir_id == hir_id
    )
}

/// Return the path last segment name.
fn path_last_segment_name(expr: &Expr<'_>) -> Option<Symbol> {
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };

    match qpath {
        QPath::Resolved(_, path) => path.segments.last().map(|segment| segment.ident.name),
        QPath::TypeRelative(_, segment) => Some(segment.ident.name),
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
