#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated accumulator and rustc syntax variants"
)]

//! A lint to check for manual for-loop accumulation where iterator adapters fit
//! better.
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

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_ast::ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    AssignOpKind, BinOpKind, BindingMode, Block, ByRef, Expr, ExprKind, HirId, MatchSource,
    Mutability, Pat, PatKind, Stmt, StmtKind,
    def::Res,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::{
    hir::nested_filter,
    ty::{self, Ty, TyCtxt},
};
use rustc_span::{Span, Symbol, sym};

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
        // Keep the accumulator adjacent to its loop, with each pair exactly two statements.
        block
            .stmts
            .array_windows()
            .filter_map(|[init_stmt, loop_stmt]| manual_iterator_loop(cx, init_stmt, loop_stmt))
            .for_each(|offense| {
                // Report each loop after its adjacent accumulator determines the rewrite.
                emit_span_lint_with_help(
                    cx,
                    MANUAL_ITERATOR_LOOP,
                    offense.span,
                    offense.message,
                    offense.help,
                );
            });
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
    // Resolve the following loop before classifying its accumulator action.
    let loop_info = support::for_loop(cx, stmt_expr(current)?)?;
    let (message, help) = matching_operation(cx, loop_info.body, &accumulator)?;
    Some(ManualLoop {
        span: loop_info.span,
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
    // Require an adjacent mutable local with a direct initializer. Skip
    // macro-generated accumulators, which are not source the reader can rewrite.
    let local = match stmt.kind {
        StmtKind::Let(local) if !local.span.from_expansion() => local,
        _ => return None,
    };

    let hir_id = mutable_binding(local.pat)?;
    let init = local.init?;
    let ty = cx.typeck_results().pat_ty(local.pat);
    // Classify only empty Vec, zero count, false any, or true all accumulators.
    let kind = validate_accumulator_kind(cx, init, ty)?;

    // Preserve the resolved binding identity for loop-body matching.
    Some(Accumulator { hir_id, kind })
}

/// Classifies one supported accumulator initializer.
fn validate_accumulator_kind(
    cx: &LateContext<'_>,
    init: &Expr<'_>,
    ty: Ty<'_>,
) -> Option<AccumulatorKind> {
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

/// Helper for loop pushes to vec analysis.
fn loop_pushes_to_vec<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    accumulator: &Accumulator,
) -> bool {
    let Some(expr) = single_body_expr(body) else {
        return false;
    };
    let ExprKind::MethodCall(_, receiver, [arg], _) = expr.kind else {
        return false;
    };

    // Require the receiver to be the resolved accumulator binding and the call to be standard
    // `Vec::push`, so local lookalike `push` methods stay quiet.
    path_is_binding(cx, receiver, accumulator.hir_id)
        && resolved_vec_push(cx, expr)
        && is_independent_expression(cx, arg, accumulator.hir_id)
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
    is_independent_expression(cx, condition, accumulator.hir_id)
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
    is_independent_expression(cx, condition, accumulator.hir_id)
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

    // Aliases and renamed imports resolve to the same diagnostic `Vec::new` item.
    cx.tcx.is_diagnostic_item(Symbol::intern("vec_new"), def_id)
}

/// Return whether resolution found vec push.
fn resolved_vec_push(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };

    // `Vec::push` has no diagnostic item, so require an inherent `push` on the `Vec` type.
    cx.tcx.item_name(def_id).as_str() == "push"
        && cx
            .tcx
            .inherent_impl_of_assoc(def_id)
            .is_some_and(|impl_def_id| {
                is_std_vec(
                    cx,
                    cx.tcx
                        .type_of(impl_def_id)
                        .instantiate_identity()
                        .skip_norm_wip(),
                )
            })
}

/// Return whether a type is the standard `Vec`.
fn is_std_vec(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    cx.tcx.is_diagnostic_item(sym::Vec, adt.did())
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

/// Requires a closure-compatible expression that never reads the changing accumulator.
fn is_independent_expression<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    accumulator: HirId,
) -> bool {
    ControlFlowFinder.visit_expr(expr).is_continue()
        && !has_accumulator_reference(cx, expr, accumulator)
}

/// Searches the expression and nested closures for the accumulator's resolved binding.
fn has_accumulator_reference<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    accumulator: HirId,
) -> bool {
    // Resolve captured paths as well as direct reads, keeping shadowed locals distinct.
    let mut finder = AccumulatorUseFinder { cx, accumulator };
    finder.visit_expr(expr).is_break()
}

/// Stops when a direct or captured path reads the accumulator.
struct AccumulatorUseFinder<'cx, 'tcx> {
    /// Compiler context used to resolve path bindings.
    cx: &'cx LateContext<'tcx>,
    /// Binding whose value changes during the original loop.
    accumulator: HirId,
}

impl<'tcx> Visitor<'tcx> for AccumulatorUseFinder<'_, 'tcx> {
    /// Visits captured reads inside nested closure bodies.
    type NestedFilter = nested_filter::OnlyBodies;
    /// Stops after the first read of the changing accumulator.
    type Result = std::ops::ControlFlow<()>;

    /// Returns the compiler context needed to enter nested bodies.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Resolves local paths before visiting their child expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        if let ExprKind::Path(qpath) = expr.kind
            && matches!(self.cx.qpath_res(&qpath, expr.hir_id), Res::Local(id) if id == self.accumulator)
        {
            return std::ops::ControlFlow::Break(());
        }
        walk_expr(self, expr)
    }
}

/// Stops on work whose control flow or mutation would change in an iterator closure.
///
/// `break`, `continue`, `return`, `?`, `.await`, and `yield` target the enclosing
/// loop or function and would target the closure instead. Loops, inline assembly,
/// and assignments keep the loop body from being a plain predicate or mapped
/// value. Nested closures are skipped because their control flow already targets
/// the closure itself.
struct ControlFlowFinder;

impl<'tcx> Visitor<'tcx> for ControlFlowFinder {
    type Result = std::ops::ControlFlow<()>;

    /// Stop on control flow and mutation; otherwise keep walking outside nested
    /// bodies.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> Self::Result {
        match expr.kind {
            ExprKind::Break(..)
            | ExprKind::Continue(..)
            | ExprKind::Ret(..)
            | ExprKind::Become(..)
            | ExprKind::Yield(..)
            | ExprKind::Loop(..)
            | ExprKind::InlineAsm(..)
            | ExprKind::Assign(..)
            | ExprKind::AssignOp(..)
            | ExprKind::Match(_, _, MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar) => {
                std::ops::ControlFlow::Break(())
            }
            _ => walk_expr(self, expr),
        }
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
