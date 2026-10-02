#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated accumulator and rustc syntax variants"
)]

//! A lint to check for manual for-loop accumulation where iterator adapters fit better.
//!
//! It classifies a small set of resolved accumulator patterns, checks the loop
//! body against the matching operation, and reports only source shapes with a
//! direct iterator equivalent. The conservative analysis preserves binding
//! identity and ignores control flow or syntax it cannot translate safely.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    AssignOpKind, BinOpKind, BindingMode, Block, ByRef, Expr, ExprKind, HirId, LoopSource,
    MatchSource, Mutability, Pat, PatKind, Stmt, StmtKind, def::Res,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ITERATOR_LOOP,
    Warn,
    "manual for-loop accumulation could use iterator adapters",
    ManualIteratorLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualIteratorLoop {
    /// Check block for this lint.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        for pair in block.stmts.windows(2) {
            // Keep the lint local: the accumulator must be initialized immediately before the loop.
            let Some([init_stmt, loop_stmt]) = pair.first_chunk::<2>() else {
                continue;
            };
            if let Some(offense) = manual_iterator_loop(cx, init_stmt, loop_stmt) {
                // Report the loop only after its adjacent accumulator determines the rewrite.
                emit_span_lint_with_help(
                    cx,
                    MANUAL_ITERATOR_LOOP,
                    offense.span,
                    offense.message,
                    offense.help,
                );
            }
        }
    }
}

/// State used by the manual loop analysis.
struct ManualLoop {
    /// span stored for this lint's analysis.
    span: Span,
    /// message stored for this lint's analysis.
    message: &'static str,
    /// help stored for this lint's analysis.
    help: &'static str,
}

/// Classification used by the accumulator kind analysis.
#[derive(Clone, Copy)]
enum AccumulatorKind {
    /// vec case used by this lint's analysis.
    Vec,
    /// count case used by this lint's analysis.
    Count,
    /// any case used by this lint's analysis.
    Any,
    /// all case used by this lint's analysis.
    All,
}

/// State used by the accumulator analysis.
struct Accumulator {
    /// hir id stored for this lint's analysis.
    hir_id: HirId,
    /// kind stored for this lint's analysis.
    kind: AccumulatorKind,
}

/// Helper for manual iterator loop analysis.
fn manual_iterator_loop<'tcx>(
    cx: &LateContext<'tcx>,
    previous: &'tcx Stmt<'tcx>,
    current: &'tcx Stmt<'tcx>,
) -> Option<ManualLoop> {
    let accumulator = accumulator_init(cx, previous)?;
    let (span, body) = for_loop_body(cx, current)?;
    let (message, help) = matching_operation(cx, body, &accumulator)?;
    Some(ManualLoop {
        span,
        message,
        help,
    })
}

/// Returns the diagnostic text for one recognized accumulator operation.
fn matching_operation<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    accumulator: &Accumulator,
) -> Option<(&'static str, &'static str)> {
    match accumulator.kind {
        AccumulatorKind::Vec => loop_pushes_to_vec(cx, body, accumulator).then_some((
            "for loop manually fills a collection",
            "use iterator adapters such as `.map(...)`, `.filter_map(...)`, and `.collect()`",
        )),
        AccumulatorKind::Count => loop_counts_matches(cx, body, accumulator).then_some((
            "for loop manually counts matching items",
            "use `.filter(...).count()` instead of incrementing a counter in the loop",
        )),
        AccumulatorKind::Any => loop_sets_bool(cx, body, accumulator, true).then_some((
            "for loop manually searches for a matching item",
            "use `.any(...)` instead of setting a boolean accumulator",
        )),
        AccumulatorKind::All => loop_sets_bool(cx, body, accumulator, false).then_some((
            "for loop manually checks whether all items match",
            "use `.all(...)` instead of clearing a boolean accumulator",
        )),
    }
}

/// Helper for accumulator init analysis.
fn accumulator_init<'tcx>(cx: &LateContext<'tcx>, stmt: &'tcx Stmt<'tcx>) -> Option<Accumulator> {
    // Require an adjacent mutable local with a direct initializer.
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };

    let hir_id = mutable_binding(local.pat)?;
    let init = local.init?;
    let ty = cx.typeck_results().pat_ty(local.pat);
    // Classify only empty Vec, zero count, false any, or true all accumulators.
    let kind = accumulator_kind(cx, init, ty)?;

    // Preserve the resolved binding identity for loop-body matching.
    Some(Accumulator { hir_id, kind })
}

/// Classifies one supported accumulator initializer.
fn accumulator_kind(cx: &LateContext<'_>, init: &Expr<'_>, ty: Ty<'_>) -> Option<AccumulatorKind> {
    vec_new_call(cx, init)
        .then_some(AccumulatorKind::Vec)
        .filter(|_| is_std_vec(cx, ty))
        .or_else(|| {
            zero_integer(init)
                .then_some(AccumulatorKind::Count)
                .filter(|_| is_integer(ty))
        })
        .or_else(|| {
            bool_literal(init, false)
                .then_some(AccumulatorKind::Any)
                .filter(|_| ty.is_bool())
        })
        .or_else(|| {
            bool_literal(init, true)
                .then_some(AccumulatorKind::All)
                .filter(|_| ty.is_bool())
        })
}

/// Helper for mutable binding analysis.
const fn mutable_binding(pat: &Pat<'_>) -> Option<HirId> {
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Mut), hir_id, _ident, None) = pat.kind
    else {
        return None;
    };

    Some(hir_id)
}

/// Helper for for loop body analysis.
fn for_loop_body<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
) -> Option<(Span, &'tcx Block<'tcx>)> {
    // Peel the statement expression and match rustc's standard for-loop shell.
    let expr = peel_drop_temps(stmt_expr(stmt)?);
    match expr.kind {
        ExprKind::Match(iter_expr, [iter_arm], MatchSource::ForLoopDesugar) => {
            Some((expr, iter_expr, iter_arm))
        }
        _ => None,
    }
    .filter(|(_, iter_expr, _)| standard_into_iter_call(cx, iter_expr))
    .and_then(|(expr, _, iter_arm)| match iter_arm.body.kind {
        ExprKind::Loop(loop_block, _label, LoopSource::ForLoop, _span) => {
            Some((expr.span, loop_block))
        }
        _ => None,
    })
    .and_then(|(span, loop_block)| {
        single_body_expr(loop_block).and_then(|expr| match expr.kind {
            ExprKind::Match(_, arms, MatchSource::ForLoopDesugar) => Some((span, arms)),
            _ => None,
        })
    })
    .and_then(|(span, arms)| {
        arms.iter().find_map(|arm| match arm.body.kind {
            ExprKind::Block(body, _) => Some((span, body)),
            _ => None,
        })
    })
}

/// Helper for loop pushes to vec analysis.
fn loop_pushes_to_vec<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    accumulator: &Accumulator,
) -> bool {
    let Some(expr) = single_body_expr(body) else {
        return false;
    };
    let ExprKind::MethodCall(segment, receiver, [arg], _) = expr.kind else {
        return false;
    };

    // Require the receiver to be the resolved accumulator binding and the call to be standard
    // `Vec::push`, so local lookalike `push` methods stay quiet.
    segment.ident.name.as_str() == "push"
        && path_is_binding(cx, receiver, accumulator.hir_id)
        && resolved_vec_push(cx, expr)
        && !contains_control_flow(arg)
}

/// Return whether this is the standard into iter call shape.
fn standard_into_iter_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::Call(callee, [_source]) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(&qpath, callee.hir_id) else {
        return false;
    };

    // A `for` loop source is iterator-shaped when rustc lowered it through the standard
    // `IntoIterator::into_iter` lang item rather than a syntactic method lookalike.
    let path = cx.tcx.def_path_str(def_id);
    path.ends_with("::into_iter") && path.contains("IntoIterator")
}

/// Helper for loop counts matches analysis.
fn loop_counts_matches<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    accumulator: &Accumulator,
) -> bool {
    let Some((condition, then_block)) = single_if_without_else(body) else {
        return false;
    };

    // Keep side-effect-heavy predicates out of this lint; the accepted shape maps directly to
    // `filter(...).count()`.
    !contains_control_flow(condition)
        && cx.typeck_results().expr_ty(condition).is_bool()
        && block_only_expr(then_block).is_some_and(|expr| increments(cx, expr, accumulator))
}

/// Helper for loop sets bool analysis.
fn loop_sets_bool<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    accumulator: &Accumulator,
    value: bool,
) -> bool {
    let Some((condition, then_block)) = single_if_without_else(body) else {
        return false;
    };

    // The boolean forms are intended to map directly to `any` or `all`.
    !contains_control_flow(condition)
        && cx.typeck_results().expr_ty(condition).is_bool()
        && block_only_expr(then_block)
            .is_some_and(|expr| assigns_bool(cx, expr, accumulator, value))
}

/// Helper for single if without else analysis.
fn single_if_without_else<'tcx>(
    body: &'tcx Block<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Block<'tcx>)> {
    // Require a single body expression with an if and no else branch.
    let expr = single_body_expr(body)?;
    let ExprKind::If(condition, then_expr, None) = expr.kind else {
        return None;
    };
    // Return only a block body that can contain one accumulator action.
    let ExprKind::Block(then_block, _) = then_expr.kind else {
        return None;
    };

    Some((condition, then_block))
}

/// Helper for single body expr analysis.
const fn single_body_expr<'tcx>(body: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    block_only_expr(body)
}

/// Helper for block only expr analysis.
const fn block_only_expr<'tcx>(block: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        ([stmt], None) => stmt_expr(stmt),
        ([], Some(expr)) => Some(expr),
        _ => None,
    }
}

/// Helper for stmt expr analysis.
const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Helper for peel drop temps analysis.
fn peel_drop_temps<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Helper for increments analysis.
fn increments(cx: &LateContext<'_>, expr: &Expr<'_>, accumulator: &Accumulator) -> bool {
    match expr.kind {
        ExprKind::AssignOp(op, lhs, rhs) => {
            op.node == AssignOpKind::AddAssign
                && path_is_binding(cx, lhs, accumulator.hir_id)
                && one_integer(rhs)
        }
        ExprKind::Assign(lhs, rhs, _) => {
            path_is_binding(cx, lhs, accumulator.hir_id)
                && adds_one_to_binding(cx, rhs, accumulator.hir_id)
        }
        _ => false,
    }
}

/// Helper for adds one to binding analysis.
fn adds_one_to_binding(cx: &LateContext<'_>, expr: &Expr<'_>, hir_id: HirId) -> bool {
    let ExprKind::Binary(op, lhs, rhs) = expr.kind else {
        return false;
    };

    op.node == BinOpKind::Add
        && ((path_is_binding(cx, lhs, hir_id) && one_integer(rhs))
            || (one_integer(lhs) && path_is_binding(cx, rhs, hir_id)))
}

/// Helper for assigns bool analysis.
fn assigns_bool(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    accumulator: &Accumulator,
    value: bool,
) -> bool {
    let ExprKind::Assign(lhs, rhs, _) = expr.kind else {
        return false;
    };

    path_is_binding(cx, lhs, accumulator.hir_id)
        && cx.typeck_results().expr_ty(rhs).is_bool()
        && bool_literal(rhs, value)
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

/// Helper for vec new call analysis.
fn vec_new_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve a zero-argument constructor path before comparing its definition.
    let ExprKind::Call(callee, []) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };

    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(&qpath, callee.hir_id) else {
        return false;
    };

    // Require both the `new` method and Vec type identity.
    let path = cx.tcx.def_path_str(def_id);

    (path.ends_with("::new") || path == "alloc::vec::Vec<T>::new")
        && (path.contains("::vec::Vec") || path.starts_with("alloc::vec::Vec"))
}

/// Return whether resolution found vec push.
fn resolved_vec_push(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };

    let path = cx.tcx.def_path_str(def_id);

    (path.ends_with("::push") || path == "alloc::vec::Vec<T>::push")
        && (path.contains("::vec::Vec") || path.starts_with("alloc::vec::Vec"))
}

/// Return whether std vec.
fn is_std_vec(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    cx.tcx.item_name(adt.did()).as_str() == "Vec"
        && matches!(cx.tcx.crate_name(adt.did().krate).as_str(), "alloc" | "std")
}

/// Return whether integer.
fn is_integer(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Int(_) | ty::Uint(_))
}

/// Helper for zero integer analysis.
fn zero_integer(expr: &Expr<'_>) -> bool {
    let ExprKind::Lit(lit) = expr.kind else {
        return false;
    };

    matches!(lit.node, LitKind::Int(value, _) if value.get() == 0)
}

/// Helper for one integer analysis.
fn one_integer(expr: &Expr<'_>) -> bool {
    let ExprKind::Lit(lit) = expr.kind else {
        return false;
    };

    matches!(lit.node, LitKind::Int(value, _) if value.get() == 1)
}

/// Helper for bool literal analysis.
const fn bool_literal(expr: &Expr<'_>, value: bool) -> bool {
    let ExprKind::Lit(lit) = expr.kind else {
        return false;
    };

    matches!(lit.node, LitKind::Bool(literal) if literal == value)
}

/// Return whether the source contains control flow.
fn contains_control_flow(expr: &Expr<'_>) -> bool {
    // Reject direct exits, loops, assembly, and assignments at any depth.
    match expr.kind {
        ExprKind::Break(..)
        | ExprKind::Continue(..)
        | ExprKind::Ret(..)
        | ExprKind::Loop(..)
        | ExprKind::InlineAsm(..)
        | ExprKind::Assign(..)
        | ExprKind::AssignOp(..) => true,
        // Recurse through the expression forms accepted by predicate analysis.
        ExprKind::Call(callee, args) => contains_call_control_flow(callee, args),
        ExprKind::MethodCall(_, receiver, args, _) => contains_call_control_flow(receiver, args),
        ExprKind::Binary(_, lhs, rhs) | ExprKind::Index(lhs, rhs, _) => {
            contains_binary_control_flow(lhs, rhs)
        }
        ExprKind::Unary(_, inner)
        | ExprKind::Cast(inner, _)
        | ExprKind::Type(inner, _)
        | ExprKind::Use(inner, _)
        | ExprKind::DropTemps(inner)
        | ExprKind::Field(inner, _)
        | ExprKind::AddrOf(_, _, inner) => contains_control_flow(inner),
        ExprKind::Block(block, _) => contains_block_control_flow(block),
        ExprKind::If(condition, then_expr, else_expr) => {
            contains_if_control_flow(condition, then_expr, else_expr)
        }
        _ => false,
    }
}

/// Returns whether a call and its arguments contain control flow.
fn contains_call_control_flow(callee: &Expr<'_>, args: &[Expr<'_>]) -> bool {
    contains_control_flow(callee) || args.iter().any(contains_control_flow)
}

/// Returns whether two expression children contain control flow.
fn contains_binary_control_flow(left: &Expr<'_>, right: &Expr<'_>) -> bool {
    contains_control_flow(left) || contains_control_flow(right)
}

/// Returns whether a block contains control flow in statements or its tail.
fn contains_block_control_flow(block: &Block<'_>) -> bool {
    block
        .stmts
        .iter()
        .any(|stmt| stmt_expr(stmt).is_none_or(contains_control_flow))
        || block.expr.is_some_and(contains_control_flow)
}

/// Returns whether an if expression contains control flow in any branch.
fn contains_if_control_flow(
    condition: &Expr<'_>,
    then_expr: &Expr<'_>,
    else_expr: Option<&Expr<'_>>,
) -> bool {
    contains_control_flow(condition)
        || contains_control_flow(then_expr)
        || else_expr.is_some_and(contains_control_flow)
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
