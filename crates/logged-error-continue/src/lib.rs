#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the lint targets control-flow forms and configures rustc diagnostics in place"
)]

//! A lint to check for logged errors that should be propagated.
//!
//! It resolves logging macros and functions in `Err` branches, compares the
//! caught error type with the enclosing function's error type, and reports
//! branches that only log before continuing. `async fn` bodies are checked
//! through their desugared coroutine, whose return type is the written `Result`.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

/// The UI examples depend on `tracing` to exercise real logging macro expansions.
#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Arm, Block, Body, Expr, ExprKind, FnDecl, LangItem, Pat, PatKind, Stmt, StmtKind,
    def::{CtorOf, DefKind, Res},
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    ExpnKind, MacroKind, Span,
    def_id::{DefId, LocalDefId},
    sym,
};

/// Crates and module names whose logging macros and functions the lint recognizes.
const LOG_NAMESPACES: &[&str] = &["kslog", "log", "tracing"];
/// Event macro names exported by the `log` and `tracing` crates.
const LOG_MACROS: &[&str] = &["debug", "error", "event", "info", "log", "trace", "warn"];
/// Logging function names recognized inside a logging namespace.
const LOG_FUNCTION_NAMES: &[&str] = &[
    "debug",
    "error",
    "info",
    "log_error",
    "log_warn",
    "trace",
    "warn",
];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LOGGED_ERROR_CONTINUE,
    Warn,
    "caught error is logged and then ignored",
    LoggedErrorContinue
}

impl<'tcx> LateLintPass<'tcx> for LoggedErrorContinue {
    /// Check each named function or method that returns `Result`.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }

        // An `async fn` body is a coroutine; its return type and statements live inside it.
        let (returned_ty, value) = if let ExprKind::Closure(closure) = body.value.kind
            && let ty::Coroutine(_, args) = cx.typeck_results().expr_ty(body.value).kind()
        {
            (
                args.as_coroutine().return_ty(),
                cx.tcx.hir_body(closure.body).value,
            )
        } else {
            (
                cx.tcx
                    .fn_sig(local_def_id)
                    .instantiate_identity()
                    .skip_norm_wip()
                    .output()
                    .skip_binder(),
                body.value,
            )
        };
        let Some(function_error_ty) = result_error_ty(cx, returned_ty) else {
            return;
        };

        // Carry the enclosing function error type so caught errors are only flagged when `?` can
        // propagate the same error type directly.
        LoggedErrorVisitor {
            cx,
            function_error_ty,
        }
        .visit_expr(value);
    }
}

/// Visitor that reports log-only `Err` branches outside nested closures.
struct LoggedErrorVisitor<'cx, 'tcx> {
    /// Lint context used for type and resolution queries.
    cx: &'cx LateContext<'tcx>,
    /// Error type of the enclosing function's `Result`.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for LoggedErrorVisitor<'_, 'tcx> {
    /// Report a log-only error branch, then continue into child expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let Some(span) = ignored_logged_error(self.cx, self.function_error_ty, expr) {
            self.cx.emit_span_lint(
                LOGGED_ERROR_CONTINUE,
                span,
                DiagDecorator(|diag| {
                    let _ = diag.primary_message(
                        "this error branch only logs the failure and then continues",
                    );
                    let _ =
                        diag.help("return the error or use `?` so the caller receives the failure");
                }),
            );
        }

        // `?` inside a closure cannot return from the enclosing function.
        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// Return the pattern span of a log-only `Err` branch in an `if let` or `match`.
fn ignored_logged_error<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<Span> {
    match expr.kind {
        ExprKind::If(condition, then_branch, _else_branch) => {
            // Require an `Err` pattern, the enclosing error type, and a log-only body.
            let ExprKind::Let(let_expr) = condition.kind else {
                return None;
            };
            (err_pattern(cx, let_expr.pat)
                && same_result_error_type(cx, function_error_ty, let_expr.init)
                && expr_only_logs(cx, then_branch))
            .then_some(let_expr.pat.span)
        }
        ExprKind::Match(scrutinee, arms, _source) => {
            ignored_match_error(cx, function_error_ty, scrutinee, arms)
        }
        _ => None,
    }
}

/// Return the pattern span of the first unguarded log-only `Err` arm.
fn ignored_match_error<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    scrutinee: &'tcx Expr<'tcx>,
    arms: &'tcx [Arm<'tcx>],
) -> Option<Span> {
    if !same_result_error_type(cx, function_error_ty, scrutinee) {
        return None;
    }

    arms.iter()
        .find(|arm| arm.guard.is_none() && err_pattern(cx, arm.pat) && expr_only_logs(cx, arm.body))
        .map(|arm| arm.pat.span)
}

/// Return whether the expression is a `Result` with the function's error type.
fn same_result_error_type<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> bool {
    result_error_ty(cx, cx.typeck_results().expr_ty(expr)) == Some(function_error_ty)
}

/// Return the error argument of a standard `Result` type.
fn result_error_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };

    cx.tcx
        .is_diagnostic_item(sym::Result, adt.did())
        .then(|| args.type_at(1))
}

/// Return whether the pattern is `Err(binding)` resolved to `Result::Err`.
fn err_pattern(cx: &LateContext<'_>, pat: &Pat<'_>) -> bool {
    let PatKind::TupleStruct(qpath, [_field], _) = pat.kind else {
        return false;
    };

    matches!(
        cx.typeck_results().qpath_res(&qpath, pat.hir_id),
        Res::Def(DefKind::Ctor(CtorOf::Variant, _), ctor_id)
            if cx.tcx.is_lang_item(cx.tcx.parent(ctor_id), LangItem::ResultErr)
    )
}

/// Return whether the branch body consists only of logging calls.
fn expr_only_logs(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if is_logging_call(cx, expr) {
        return true;
    }

    match expr.kind {
        ExprKind::Block(block, _) => block_only_logs(cx, block),
        _ => false,
    }
}

/// Return whether a non-empty block contains only logging statements.
fn block_only_logs(cx: &LateContext<'_>, block: &Block<'_>) -> bool {
    let stmts_only_log = block.stmts.iter().all(|stmt| stmt_only_logs(cx, stmt));
    let tail_only_logs = block.expr.is_none_or(|expr| expr_only_logs(cx, expr));

    // Empty blocks do not log anything, so require at least one logging expression.
    stmts_only_log && tail_only_logs && (!block.stmts.is_empty() || block.expr.is_some())
}

/// Return whether a statement is a logging expression.
fn stmt_only_logs(cx: &LateContext<'_>, stmt: &Stmt<'_>) -> bool {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr_only_logs(cx, expr),
        StmtKind::Let(_) | StmtKind::Item(_) => false,
    }
}

/// Return whether the expression is a recognized logging macro or function call.
fn is_logging_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_logging_macro_call(cx, expr.span) || logging_function_call(cx, expr)
}

/// Return whether any macro expansion that produced the span is a logging macro.
fn is_logging_macro_call(cx: &LateContext<'_>, span: Span) -> bool {
    span.macro_backtrace().any(|expn_data| {
        matches!(expn_data.kind, ExpnKind::Macro(MacroKind::Bang, _))
            && expn_data
                .macro_def_id
                .is_some_and(|def_id| is_logging_macro(cx, def_id))
    })
}

/// Return whether a macro definition is `eprintln!` or a `log` or `tracing` event macro.
fn is_logging_macro(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx
        .get_diagnostic_name(def_id)
        .is_some_and(|name| name.as_str() == "eprintln_macro")
        || (LOG_NAMESPACES.contains(&cx.tcx.crate_name(def_id.krate).as_str())
            && LOG_MACROS.contains(&cx.tcx.item_name(def_id).as_str()))
}

/// Return whether the expression calls a function that resolves to a logging namespace.
fn logging_function_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Only a call through a path can resolve to a named logging function.
    let ExprKind::Call(callee, _args) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.typeck_results().qpath_res(&qpath, callee.hir_id) else {
        return false;
    };

    // Resolve through imports and renames, then require a logging crate or module in the path.
    LOG_FUNCTION_NAMES.contains(&cx.tcx.item_name(def_id).as_str())
        && (LOG_NAMESPACES.contains(&cx.tcx.crate_name(def_id.krate).as_str())
            || cx.tcx.def_path(def_id).data.iter().any(|segment| {
                segment
                    .data
                    .get_opt_name()
                    .is_some_and(|name| LOG_NAMESPACES.contains(&name.as_str()))
            }))
}

/// Run the UI examples, which depend on the real `tracing` crate.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
