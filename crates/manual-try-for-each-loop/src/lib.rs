#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only targeted expression wrappers matter and rustc diagnostics are configured in place"
)]
#![warn(unused_extern_crates)]

//! A lint to check for fallible loops that can use `Iterator::try_for_each`.
//!
//! It looks at the outermost block of each function, closure, or async body.
//! When that block ends with a standard `for` loop whose body holds one `?`,
//! followed by a unit success constructor with the same residual, it reports
//! the loop. Constructors, residual types, and the loop desugaring are matched
//! through lang items and diagnostic items rather than source text.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Block, Body, Expr, ExprKind, LangItem, MatchSource, Stmt, StmtKind,
    def::{DefKind, Res},
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{DesugaringKind, Span, Symbol, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_TRY_FOR_EACH_LOOP,
    Warn,
    "fallible `for` loop can use `Iterator::try_for_each`",
    ManualTryForEachLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualTryForEachLoop {
    /// Checks a function, closure, or async body for a final fallible loop and unit
    /// success tail.
    ///
    /// Only the outermost block of a body qualifies, because `?` returns from the
    /// body. In a nested block, the loop's `?` and the block's tail value would have
    /// different targets.
    fn check_body(&mut self, cx: &LateContext<'tcx>, body: &Body<'tcx>) {
        let Some(block) = outer_block(body.value) else {
            return;
        };
        // Limit the rewrite to a final loop followed by its direct unit success tail.
        let (Some(loop_stmt), Some(tail)) = (block.stmts.last(), block.expr) else {
            return;
        };
        let Some(residual) = unit_success_residual(cx, tail) else {
            return;
        };

        // Require one standard loop whose body propagates the same residual.
        if let Some(loop_expr) = stmt_expr(loop_stmt)
            && let Some(loop_info) = support::for_loop(cx, loop_expr)
            && let Some(action) = block_only_expr(loop_info.body)
            && is_fallible_body_match(cx, action, residual)
        {
            emit_span_lint_with_help(
                cx,
                MANUAL_TRY_FOR_EACH_LOOP,
                loop_info.span,
                "fallible `for` loop can use `Iterator::try_for_each`",
                "use `Iterator::try_for_each` for this fallible loop",
            );
        }
    }
}

/// A residual type accepted by `try_for_each`.
#[derive(Clone, Copy)]
enum Residual<'tcx> {
    /// Standard `Result` with this error type.
    Result(Ty<'tcx>),
    /// Standard `Option`.
    Option,
    /// Standard `ControlFlow` with this break type.
    ControlFlow(Ty<'tcx>),
}

/// Returns the outermost block of a body, looking through blocks that only wrap a
/// block.
fn outer_block<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Block<'tcx>> {
    let ExprKind::Block(block, _) = peel_drop_temps(expr).kind else {
        return None;
    };
    // A block whose only content is another block evaluates that block in the same body.
    // An async body moves its parameters with generated `let` statements before the user's
    // block, which still runs in the same coroutine body.
    let is_async_wrapper = block.span.is_desugaring(DesugaringKind::Async)
        && block
            .stmts
            .iter()
            .all(|stmt| matches!(stmt.kind, StmtKind::Let(_)));
    match (block.stmts, block.expr) {
        ([], Some(inner)) if matches!(peel_drop_temps(inner).kind, ExprKind::Block(..)) => {
            outer_block(inner)
        }
        (_, Some(inner)) if is_async_wrapper => outer_block(inner),
        _ => Some(block),
    }
}

/// Returns the residual of an `Ok(())`, `Some(())`, or
/// `ControlFlow::Continue(())` tail.
fn unit_success_residual<'tcx>(
    cx: &LateContext<'tcx>,
    tail: &'tcx Expr<'tcx>,
) -> Option<Residual<'tcx>> {
    // Require a source-written call with a unit argument.
    let ExprKind::Call(callee, [argument]) = tail.kind else {
        return None;
    };
    let is_unit_call = !tail.span.from_expansion() && matches!(argument.kind, ExprKind::Tup([]));
    let tail_ty = cx.typeck_results().expr_ty(tail);
    let (true, ExprKind::Path(qpath), ty::Adt(_, args)) =
        (is_unit_call, callee.kind, tail_ty.kind())
    else {
        return None;
    };
    // Resolve the callee to a constructor rather than a function with the same name.
    let Res::Def(DefKind::Ctor(..), ctor_def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return None;
    };

    // Map the constructor's variant to the residual that `?` propagates.
    match cx.tcx.as_lang_item(cx.tcx.parent(ctor_def_id))? {
        LangItem::ResultOk => Some(Residual::Result(args.type_at(1))),
        LangItem::OptionSome => Some(Residual::Option),
        LangItem::ControlFlowContinue => Some(Residual::ControlFlow(args.type_at(0))),
        _ => None,
    }
}

/// Returns whether the loop body has exactly one `?` with the tail's residual and
/// no other exit.
fn is_fallible_body_match<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    residual: Residual<'tcx>,
) -> bool {
    // Collect `?` operands and reject control flow that a closure would retarget.
    let mut visitor = TryOperandVisitor {
        operands: Vec::new(),
        has_other_exit: false,
    };
    visitor.visit_expr(expr);
    if visitor.has_other_exit {
        return false;
    }
    // Require exactly one `?` whose operand already has the tail's residual type.
    let [operand] = visitor.operands.as_slice() else {
        return false;
    };
    is_residual_match(cx, cx.typeck_results().expr_ty(operand), residual)
}

/// Collects `?` operands outside nested closures and records other exits.
struct TryOperandVisitor<'tcx> {
    /// User-written operands of `?` expressions.
    operands: Vec<&'tcx Expr<'tcx>>,
    /// Set by `break`, `continue`, `return`, `become`, `yield`, or `.await`.
    has_other_exit: bool,
}

impl<'tcx> Visitor<'tcx> for TryOperandVisitor<'tcx> {
    /// Records `?` operands and exits; nested closure bodies are not visited.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)) => {
                // rustc lowers `operand?` to a match on `Try::branch(operand)` whose arms
                // contain the generated `return`; visit only the user-written operand.
                if let ExprKind::Call(_, [operand]) = scrutinee.kind {
                    self.operands.push(operand);
                    self.visit_expr(operand);
                    return;
                }
            }
            ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Ret(_)
            | ExprKind::Become(_)
            | ExprKind::Yield(..)
            | ExprKind::Match(_, _, MatchSource::AwaitDesugar) => {
                self.has_other_exit = true;
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Returns whether a type has the required standard residual.
fn is_residual_match<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, residual: Residual<'tcx>) -> bool {
    // Compare the residual while allowing the successful value type to differ.
    let ty::Adt(adt, args) = ty.kind() else {
        return false;
    };
    match residual {
        Residual::Result(error_ty) => {
            cx.tcx.is_diagnostic_item(sym::Result, adt.did()) && args.type_at(1) == error_ty
        }
        Residual::Option => cx.tcx.is_diagnostic_item(sym::Option, adt.did()),
        Residual::ControlFlow(break_ty) => {
            cx.tcx
                .is_diagnostic_item(Symbol::intern("ControlFlow"), adt.did())
                && args.type_at(0) == break_ty
        }
    }
}

/// Returns the only expression in a block.
const fn block_only_expr<'tcx>(block: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        ([stmt], None) => stmt_expr(stmt),
        ([], Some(expr)) => Some(expr),
        _ => None,
    }
}

/// Returns an expression statement.
const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Removes compiler-generated temporary wrappers.
fn peel_drop_temps<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Emits the lint diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Keep this lint help-only because closure rendering can need annotations.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
