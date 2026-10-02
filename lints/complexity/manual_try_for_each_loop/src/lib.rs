#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only targeted expression wrappers matter and rustc diagnostics are configured in place"
)]
#![warn(unused_extern_crates)]

//! A lint to check for fallible loops that can use `Iterator::try_for_each`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Block, Expr, ExprKind, MatchSource, Stmt, StmtKind,
    def::Res,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_TRY_FOR_EACH_LOOP,
    Warn,
    "fallible `for` loop can use `Iterator::try_for_each`",
    ManualTryForEachLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualTryForEachLoop {
    /// Checks a block for a final fallible loop and matching unit success tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Limit the rewrite to a final loop followed by its direct unit success tail.
        let (Some(loop_stmt), Some(tail)) = (block.stmts.last(), block.expr) else {
            return;
        };
        let Some(residual) = unit_success_residual(cx, tail) else {
            return;
        };

        // Require one standard loop whose body propagates the same residual.
        if let Some(loop_expr) = stmt_expr(loop_stmt)
            && let Some((span, body)) = for_loop_expr(cx, loop_expr)
            && let Some(action) = block_only_expr(body)
            && is_fallible_body_match(cx, action, residual)
        {
            emit_span_lint_with_help(
                cx,
                MANUAL_TRY_FOR_EACH_LOOP,
                span,
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

/// Returns the residual represented by an exact unit success tail.
fn unit_success_residual<'tcx>(
    cx: &LateContext<'tcx>,
    tail: &'tcx Expr<'tcx>,
) -> Option<Residual<'tcx>> {
    // Resolve the tail ADT and preserve its generic residual arguments.
    match cx.typeck_results().expr_ty(tail).kind() {
        ty::Adt(adt, args) => Some((*adt, args)),
        _ => None,
    }
    .and_then(|(adt, args)| {
        cx.sess()
            .source_map()
            .span_to_snippet(tail.span.source_callsite())
            .ok()
            .map(|source| (adt, args, source))
    })
    .and_then(|(adt, args, source)| constructor_residual(cx, adt, args, &source))
}

/// Return the residual represented by one supported success constructor.
fn constructor_residual<'tcx>(
    cx: &LateContext<'tcx>,
    adt: ty::AdtDef<'tcx>,
    args: ty::GenericArgsRef<'tcx>,
    source: &str,
) -> Option<Residual<'tcx>> {
    let first_ty = type_arg(args, 0)?;
    let source = source.trim_start();

    // Accept an exact unit-success Result constructor and retain its error type.
    cx.tcx
        .is_diagnostic_item(sym::Result, adt.did())
        .then_some(())
        .filter(|()| first_ty.is_unit() && source.starts_with("Ok("))
        .and_then(|()| type_arg(args, 1).map(Residual::Result))
        // Accept an exact unit-success Option constructor.
        .or_else(|| {
            cx.tcx
                .is_diagnostic_item(sym::Option, adt.did())
                .then_some(())
                .filter(|()| first_ty.is_unit() && source.starts_with("Some("))
                .map(|()| Residual::Option)
        })
        // Accept unit-continue ControlFlow and retain its break type.
        .or_else(|| {
            cx.tcx
                .def_path_str(adt.did())
                .ends_with("::ControlFlow")
                .then_some(())
                .filter(|()| {
                    type_arg(args, 1).is_some_and(Ty::is_unit) && source.contains("Continue(")
                })
                .map(|()| Residual::ControlFlow(first_ty))
        })
}

/// Returns a type argument without indexing an incomplete or erased argument list.
fn type_arg(args: ty::GenericArgsRef<'_>, index: usize) -> Option<Ty<'_>> {
    args.iter().nth(index).and_then(ty::GenericArg::as_type)
}

/// Extracts the body of a standard `for` loop desugaring.
fn for_loop_expr<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(Span, &'tcx Block<'tcx>)> {
    // Peel the temporary wrapper and match rustc's standard for-loop shell.
    let expr = peel_drop_temps(expr);
    match expr.kind {
        ExprKind::Match(iter_expr, [iter_arm], MatchSource::ForLoopDesugar) => {
            Some((expr.span, iter_expr, iter_arm))
        }
        _ => None,
    }
    .filter(|(_, iter_expr, _)| standard_into_iter_call(cx, iter_expr))
    // Descend through the generated loop and iterator match.
    .and_then(|(span, _, iter_arm)| match iter_arm.body.kind {
        ExprKind::Loop(loop_block, _, _, _) => Some((span, loop_block)),
        _ => None,
    })
    .and_then(|(span, loop_block)| {
        block_only_expr(loop_block).and_then(|expr| match expr.kind {
            ExprKind::Match(_, arms, MatchSource::ForLoopDesugar) => Some((span, arms)),
            _ => None,
        })
    })
    // Select the arm that carries the user-written block expression.
    .and_then(|(span, arms)| {
        arms.iter().find_map(|arm| match arm.body.kind {
            ExprKind::Block(body, _) => Some((span, body)),
            _ => None,
        })
    })
}

/// Returns whether the `for` loop used the standard `IntoIterator` trait.
fn standard_into_iter_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Compare the resolved trait method so a custom spelling does not trigger.
    let ExprKind::Call(callee, [_]) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return false;
    };
    // Require both the method suffix and the standard trait identity.
    let path = cx.tcx.def_path_str(def_id);
    path.ends_with("::into_iter") && path.contains("IntoIterator")
}

/// Returns whether the loop body has one matching typed fallible operation.
fn is_fallible_body_match<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    residual: Residual<'tcx>,
) -> bool {
    // Require one `?` before proving that a typed residual matches the tail.
    let Ok(source) = cx.sess().source_map().span_to_snippet(expr.span) else {
        return false;
    };
    if source.matches('?').count() != 1 {
        return false;
    }

    // Count typed residual expressions outside nested closures.
    let mut visitor = MatchingResidualVisitor {
        cx,
        residual,
        matches: 0,
    };
    visitor.visit_expr(expr);
    visitor.matches >= 1
}

/// Counts fallible expressions with the enclosing residual type.
struct MatchingResidualVisitor<'cx, 'tcx> {
    /// Compiler context used for expression types.
    cx: &'cx LateContext<'tcx>,
    /// Required residual type.
    residual: Residual<'tcx>,
    /// Number of matching expressions.
    matches: usize,
}

impl<'tcx> Visitor<'tcx> for MatchingResidualVisitor<'_, 'tcx> {
    /// Visits expressions while excluding nested closures.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Count only expressions with the exact standard residual profile.
        if is_residual_match(
            self.cx,
            self.cx.typeck_results().expr_ty(expr),
            self.residual,
        ) {
            self.matches += 1;
        }
        if !matches!(expr.kind, ExprKind::Closure(_)) {
            walk_expr(self, expr);
        }
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
            cx.tcx.def_path_str(adt.did()).ends_with("::ControlFlow") && args.type_at(0) == break_ty
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
