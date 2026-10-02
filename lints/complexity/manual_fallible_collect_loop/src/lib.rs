#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

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

use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, Mutability, PatKind, Stmt, StmtKind,
    def::Res,
    intravisit::{Visitor, walk_expr},
};
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
        // Establish the fallible success type before scanning local statement pairs.
        let Some(tail) = block.expr else {
            return;
        };
        let Some(residual) = success_residual(cx, tail) else {
            return;
        };
        // Evaluate each accumulator and its immediately following loop.
        for pair in block.stmts.windows(2) {
            let [accumulator_stmt, loop_stmt] = pair else {
                continue;
            };
            let Some(accumulator) = accumulator(cx, accumulator_stmt) else {
                continue;
            };
            // Resolve the loop only after establishing its accumulator identity.
            let Some(loop_expr) = support::stmt_expr(loop_stmt) else {
                continue;
            };
            let Some(loop_info) = support::for_loop(cx, loop_expr) else {
                continue;
            };
            let is_tail_match = tail_returns_binding(cx, tail, accumulator.id);
            let is_insertion_match =
                is_fallible_insertion(cx, loop_info.body, accumulator.id, residual);
            // Require the same accumulator in both the insertion and success tail.
            if !is_tail_match || !is_insertion_match {
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

/// One mutable standard collection accumulator.
struct Accumulator {
    /// Local binding resolved by HIR.
    id: HirId,
}

/// A collection residual accepted by standard `collect`.
#[derive(Clone, Copy)]
enum Residual<'tcx> {
    /// Standard `Result` with this error type.
    Result(Ty<'tcx>),
    /// Standard `Option`.
    Option,
}

/// Returns the residual represented by a standard success constructor.
fn success_residual<'tcx>(
    cx: &LateContext<'tcx>,
    tail: &'tcx Expr<'tcx>,
) -> Option<Residual<'tcx>> {
    // Resolve the constructor and retain the residual required by `collect`.
    let ty::Adt(adt, args) = cx.typeck_results().expr_ty(tail).kind() else {
        return None;
    };
    // Pair resolved Result or Option identity with its written success constructor.
    let source = support::snippet(cx, tail.span)?;
    if cx
        .tcx
        .is_diagnostic_item(rustc_span::sym::Result, adt.did())
        && source.trim_start().starts_with("Ok(")
    {
        Some(Residual::Result(args.type_at(1)))
    } else if cx
        .tcx
        .is_diagnostic_item(rustc_span::sym::Option, adt.did())
        && source.trim_start().starts_with("Some(")
    {
        Some(Residual::Option)
    } else {
        None
    }
}

/// Parses an empty mutable standard collection binding.
fn accumulator(cx: &LateContext<'_>, stmt: &Stmt<'_>) -> Option<Accumulator> {
    // Require a mutable empty standard collection immediately before the loop.
    match stmt.kind {
        StmtKind::Let(local) => Some(local),
        _ => None,
    }
    .and_then(|local| match local.pat.kind {
        PatKind::Binding(BindingMode(ByRef::No, Mutability::Mut), id, _, None) => Some((local, id)),
        _ => None,
    })
    // Recover the initializer and require a resolved standard collection type.
    .and_then(|(local, id)| local.init.map(|init| (local, id, init)))
    .and_then(|(local, id, init)| support::snippet(cx, init.span).map(|source| (local, id, source)))
    .filter(|(local, _, source)| {
        support::is_standard_collection(cx, cx.typeck_results().pat_ty(local.pat))
            && (source.ends_with("::new()") || source.ends_with("::default()"))
    })
    .map(|(_, id, _)| Accumulator { id })
}

/// Returns whether the success tail contains the exact accumulator binding.
fn tail_returns_binding(cx: &LateContext<'_>, tail: &Expr<'_>, id: HirId) -> bool {
    let ExprKind::Call(_, [value]) = support::peel_drop_temps(tail).kind else {
        return false;
    };
    path_is_binding(cx, value, id)
}

/// Checks one standard collection insertion containing a matching residual.
fn is_fallible_insertion<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    id: HirId,
    residual: Residual<'tcx>,
) -> bool {
    // Match one resolved insertion before inspecting its `?` residual.
    support::block_only_expr(body)
        .and_then(|action| match action.kind {
            ExprKind::MethodCall(_, receiver, [argument], _) => Some((action, receiver, argument)),
            _ => None,
        })
        // Resolve the collection method before accepting insertion vocabulary.
        .and_then(|(action, receiver, argument)| {
            support::method_path(cx, action).map(|path| (receiver, argument, path))
        })
        .filter(|(_, _, path)| {
            path.ends_with("::push") || path.ends_with("::push_back") || path.ends_with("::insert")
        })
        .filter(|(receiver, _, _)| path_is_binding(cx, receiver, id))
        .and_then(|(_, argument, _)| {
            support::snippet(cx, argument.span).map(|source| (argument, source))
        })
        // Require exactly one question-mark operation in the inserted expression.
        .filter(|(_, source)| source.matches('?').count() == 1)
        .map(|(argument, _)| {
            let mut visitor = ResultVisitor {
                cx,
                residual,
                is_found: false,
            };
            // Traverse the argument to prove that its fallible expression has the same residual.
            visitor.visit_expr(argument);
            visitor.is_found
        })
        .is_some_and(|is_found| is_found)
}

/// Finds a fallible expression with one exact residual type.
struct ResultVisitor<'cx, 'tcx> {
    /// Compiler context used for expression types.
    cx: &'cx LateContext<'tcx>,
    /// Required residual type.
    residual: Residual<'tcx>,
    /// Whether a matching result expression was found.
    is_found: bool,
}

impl<'tcx> Visitor<'tcx> for ResultVisitor<'_, 'tcx> {
    /// Visits expressions without entering nested closures.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Record any expression whose standard residual matches the block tail.
        self.is_found |= is_residual_match(
            self.cx,
            self.cx.typeck_results().expr_ty(expr),
            self.residual,
        );
        if !matches!(expr.kind, ExprKind::Closure(_)) {
            walk_expr(self, expr);
        }
    }
}

/// Returns whether a type has the required standard collection residual.
fn is_residual_match<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, residual: Residual<'tcx>) -> bool {
    // Compare the residual while allowing the collected success type to differ.
    let ty::Adt(adt, args) = ty.kind() else {
        return false;
    };
    match residual {
        Residual::Result(error_ty) => {
            cx.tcx
                .is_diagnostic_item(rustc_span::sym::Result, adt.did())
                && args.type_at(1) == error_ty
        }
        Residual::Option => cx
            .tcx
            .is_diagnostic_item(rustc_span::sym::Option, adt.did()),
    }
}

/// Returns whether an expression is one resolved local binding.
fn path_is_binding(cx: &LateContext<'_>, expr: &Expr<'_>, id: HirId) -> bool {
    let ExprKind::Path(qpath) = support::peel_drop_temps(expr).kind else {
        return false;
    };
    matches!(cx.qpath_res(&qpath, expr.hir_id), Res::Local(found) if found == id)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
