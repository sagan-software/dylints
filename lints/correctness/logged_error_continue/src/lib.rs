#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the lint targets control-flow forms and configures rustc diagnostics in place"
)]

//! A lint to check for logged errors that should be propagated.
//!
//! It resolves common logging calls in error branches, distinguishes handled
//! recovery from ignored failure, and reports control-flow paths that continue
//! after recording an error. The diagnostic recommends returning or propagating
//! the error when the surrounding function has an honest failure boundary.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Arm, Block, Body, Expr, ExprKind, FnDecl, Pat, PatKind, QPath, Stmt, StmtKind,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{ExpnKind, MacroKind, Span, sym};

/// `LOG_MACROS` configuration used by this lint.
const LOG_MACROS: &[&str] = &["debug", "eprintln", "error", "info", "trace", "warn"];
/// `LOG_FUNCTION_NAMES` configuration used by this lint.
const LOG_FUNCTION_NAMES: &[&str] = &[
    "debug",
    "error",
    "info",
    "log_error",
    "log_warn",
    "trace",
    "warn",
];
/// `LOG_MODULE_NAMES` configuration used by this lint.
const LOG_MODULE_NAMES: &[&str] = &["kslog", "log", "tracing"];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LOGGED_ERROR_CONTINUE,
    Warn,
    "caught error is logged and then ignored",
    LoggedErrorContinue
}

impl<'tcx> LateLintPass<'tcx> for LoggedErrorContinue {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        _local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }
        let Some(function_error_ty) = result_error_ty(cx, cx.typeck_results().expr_ty(body.value))
        else {
            return;
        };

        // Carry the enclosing function error type so caught errors are only flagged when `?` can
        // propagate the same error type directly.
        LoggedErrorVisitor {
            cx,
            function_error_ty,
        }
        .visit_expr(body.value);
    }
}

/// State used by the logged error visitor analysis.
struct LoggedErrorVisitor<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
    /// function error ty stored for this lint's analysis.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for LoggedErrorVisitor<'_, 'tcx> {
    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Report a matching ignored error before descending into child expressions.
        if let Some(span) = ignored_logged_error(self.cx, self.function_error_ty, expr) {
            emit_span_lint_with_help(
                self.cx,
                LOGGED_ERROR_CONTINUE,
                span,
                "this error branch only logs the failure and then continues",
                "return the error or use `?` so the caller receives the failure",
            );
        }

        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// Helper for ignored logged error analysis.
fn ignored_logged_error<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<Span> {
    match expr.kind {
        ExprKind::If(condition, then_branch, _else_branch) => {
            ignored_if_let_error(cx, function_error_ty, condition, then_branch)
        }
        ExprKind::Match(scrutinee, arms, _source) => {
            ignored_match_error(cx, function_error_ty, scrutinee, arms)
        }
        _ => None,
    }
}

/// Helper for ignored if let error analysis.
fn ignored_if_let_error<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    condition: &'tcx Expr<'tcx>,
    then_branch: &'tcx Expr<'tcx>,
) -> Option<Span> {
    // Require a simple Err pattern, the enclosing error type, and a log-only body.
    let ExprKind::Let(let_expr) = condition.kind else {
        return None;
    };
    if !err_pattern(let_expr.pat)
        || !same_result_error_type(cx, function_error_ty, let_expr.init)
        || !expr_only_logs(then_branch)
    {
        return None;
    }

    Some(let_expr.pat.span)
}

/// Helper for ignored match error analysis.
fn ignored_match_error<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    scrutinee: &'tcx Expr<'tcx>,
    arms: &'tcx [Arm<'tcx>],
) -> Option<Span> {
    if !same_result_error_type(cx, function_error_ty, scrutinee) {
        return None;
    }

    // Report the first simple `Err` arm that only logs; other arms may keep their own behavior.
    arms.iter()
        .find(|arm| arm.guard.is_none() && err_pattern(arm.pat) && expr_only_logs(arm.body))
        .map(|arm| arm.pat.span)
}

/// Return whether result error type match.
fn same_result_error_type<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> bool {
    result_error_ty(cx, cx.typeck_results().expr_ty(expr)) == Some(function_error_ty)
}

/// Return type information for result error.
fn result_error_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    // Extract the error argument only from the standard Result diagnostic item.
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    if !cx.tcx.is_diagnostic_item(sym::Result, adt.did()) {
        return None;
    }

    Some(args.type_at(1))
}

/// Helper for err pattern analysis.
fn err_pattern(pat: &Pat<'_>) -> bool {
    let PatKind::TupleStruct(qpath, fields, _) = pat.kind else {
        return false;
    };

    // `Err(error)` is the only catch shape this lint can safely propose replacing with `?`.
    fields.len() == 1 && qpath_last_segment_name(qpath).is_some_and(|name| name == "Err")
}

/// Helper for expr only logs analysis.
fn expr_only_logs(expr: &Expr<'_>) -> bool {
    if logging_call(expr) {
        return true;
    }

    match expr.kind {
        ExprKind::Block(block, _) => block_only_logs(block),
        _ => false,
    }
}

/// Helper for block only logs analysis.
fn block_only_logs(block: &Block<'_>) -> bool {
    let stmts_only_log = block.stmts.iter().all(stmt_only_logs);
    let tail_only_logs = block.expr.is_none_or(expr_only_logs);

    // Empty blocks do not log anything, so require at least one logging expression.
    stmts_only_log && tail_only_logs && (!block.stmts.is_empty() || block.expr.is_some())
}

/// Helper for stmt only logs analysis.
fn stmt_only_logs(stmt: &Stmt<'_>) -> bool {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => expr_only_logs(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => false,
    }
}

/// Helper for logging call analysis.
fn logging_call(expr: &Expr<'_>) -> bool {
    logging_macro_call(expr.span) || logging_function_call(expr)
}

/// Helper for logging macro call analysis.
fn logging_macro_call(span: Span) -> bool {
    let mut expn_data = span.ctxt().outer_expn_data();

    // Macro-expanded logging calls can lower to several HIR nodes; use the outer call site name.
    loop {
        if let ExpnKind::Macro(MacroKind::Bang, name) = expn_data.kind {
            let macro_name = normalize_path_name(name.as_str());
            if LOG_MACROS.contains(&macro_name) {
                return true;
            }
        }

        if !expn_data.call_site.from_expansion() {
            return false;
        }

        // Walk outward until a recognized logging macro or source call site appears.
        expn_data = expn_data.call_site.ctxt().outer_expn_data();
    }
}

/// Helper for logging function call analysis.
fn logging_function_call(expr: &Expr<'_>) -> bool {
    let ExprKind::Call(callee, _args) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let names = qpath_names(qpath);
    let Some(last_name) = names.last().map(String::as_str) else {
        return false;
    };

    // Keep function-call matching namespaced so ordinary `warn(...)` helpers remain valid.
    // Require both a logging function name and a known logging module segment.
    LOG_FUNCTION_NAMES.contains(&last_name)
        && names
            .iter()
            .any(|name| LOG_MODULE_NAMES.contains(&name.as_str()))
}

/// Return the qpath last segment name.
fn qpath_last_segment_name(qpath: QPath<'_>) -> Option<String> {
    match qpath {
        QPath::Resolved(_, path) => path.segments.last(),
        QPath::TypeRelative(_, segment) => Some(segment),
    }
    .map(|segment| segment.ident.name.to_ident_string())
}

/// Helper for qpath names analysis.
fn qpath_names(qpath: QPath<'_>) -> Vec<String> {
    match qpath {
        QPath::Resolved(_, path) => path
            .segments
            .iter()
            .map(|segment| segment.ident.name.to_ident_string())
            .collect(),
        QPath::TypeRelative(_, segment) => vec![segment.ident.name.to_ident_string()],
    }
}

/// Return the normalized path name.
fn normalize_path_name(name: &str) -> &str {
    name.rsplit("::")
        .next()
        .unwrap_or(name)
        .strip_suffix("_macro")
        .unwrap_or_else(|| name.rsplit("::").next().unwrap_or(name))
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to match the rest of this lint suite.
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
