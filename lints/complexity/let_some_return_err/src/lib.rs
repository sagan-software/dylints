#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to replace let-else `Option` error returns with `ok_or`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Block, Body, Expr, ExprKind, FnDecl, LetStmt, Pat, PatKind, QPath, Stmt, StmtKind,
    def::Res,
    intravisit::{FnKind, Visitor, walk_expr, walk_stmt},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LET_SOME_RETURN_ERR,
    Warn,
    "`let Some` else branch returns `Err` manually",
    LetSomeReturnErr
}

impl<'tcx> LateLintPass<'tcx> for LetSomeReturnErr {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        _local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        let Some(function_error_ty) = result_error_ty(cx, cx.typeck_results().expr_ty(body.value))
        else {
            return;
        };

        // Carry the enclosing `Result` error type so nested `let else` statements are only
        // considered where `?` can preserve the current function or closure error type.
        LetSomeVisitor {
            cx,
            function_error_ty,
        }
        .visit_expr(body.value);
    }
}

/// State used by the let some visitor analysis.
struct LetSomeVisitor<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
    /// function error ty stored for this lint's analysis.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for LetSomeVisitor<'_, 'tcx> {
    /// Helper for visit stmt analysis.
    fn visit_stmt(&mut self, stmt: &'tcx Stmt<'tcx>) {
        // Analyze let statements while preserving traversal of every other statement.
        let StmtKind::Let(let_stmt) = stmt.kind else {
            walk_stmt(self, stmt);
            return;
        };

        // Emit a source-preserving replacement when the complete pattern matches.
        if let Some(suggestion) = let_some_returns_err(self.cx, self.function_error_ty, let_stmt) {
            emit_span_lint_with_help(
                self.cx,
                LET_SOME_RETURN_ERR,
                let_stmt.span,
                "`Option` mismatch returns `Err` manually",
                "convert the initializer with `.ok_or(...)` or `.ok_or_else(...)` and use `?`",
                Some(suggestion),
            );
        }

        walk_stmt(self, stmt);
    }

    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// Helper for let some returns err analysis.
fn let_some_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    let_stmt: &'tcx LetStmt<'tcx>,
) -> Option<String> {
    // Require the initializer, Some pattern, else block, and returned error together.
    let_stmt
        .init
        .zip(some_option_pattern(cx, let_stmt.pat))
        .and_then(|(init, pattern)| {
            let_stmt.els.and_then(|els| {
                block_only_returns_err(cx, function_error_ty, els)
                    .map(|error| (init, pattern, error))
            })
        })
        // Build the replacement from source snippets so `--fix` preserves the user's expressions.
        .and_then(|(init, pattern, error)| {
            let source_map = cx.sess().source_map();
            source_map
                .span_to_snippet(pattern.span)
                .ok()
                .and_then(|pattern| {
                    source_map.span_to_snippet(init.span).ok().and_then(|init| {
                        source_map
                            .span_to_snippet(error.span)
                            .ok()
                            .map(|error| (pattern, init, error))
                    })
                })
        })
        // Parenthesize the initializer so method-call precedence remains stable.
        .map(|(pattern, init, error)| format!("let {pattern} = ({init}).ok_or_else(|| {error})?;"))
}

/// Helper for some option pattern analysis.
fn some_option_pattern<'tcx>(
    cx: &LateContext<'tcx>,
    pat: &'tcx Pat<'tcx>,
) -> Option<&'tcx Pat<'tcx>> {
    // Require a one-field tuple-struct pattern before checking its resolved Option type.
    let PatKind::TupleStruct(_qpath, subpats, _) = pat.kind else {
        return None;
    };

    if subpats.len() == 1 && option_pat_ty(cx, pat) {
        subpats.first()
    } else {
        None
    }
}

/// Return type information for option pat.
fn option_pat_ty<'tcx>(cx: &LateContext<'tcx>, pat: &Pat<'tcx>) -> bool {
    let ty::Adt(adt, _) = cx.typeck_results().pat_ty(pat).kind() else {
        return false;
    };

    cx.tcx.is_diagnostic_item(sym::Option, adt.did())
}

/// Helper for block only returns err analysis.
fn block_only_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    block: &'tcx Block<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        // Match both `return Err(..);` and `return Err(..)` forms inside the else block.
        ([], Some(expr)) => returns_err(cx, function_error_ty, expr),
        ([stmt], None) => stmt_returns_err(cx, function_error_ty, stmt),
        _ => None,
    }
}

/// Helper for stmt returns err analysis.
fn stmt_returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => returns_err(cx, function_error_ty, expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Return whether the item returns err.
fn returns_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    let ExprKind::Ret(Some(returned)) = expr.kind else {
        return None;
    };

    err_call(cx, function_error_ty, returned)
}

/// Helper for err call analysis.
fn err_call<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Require a one-argument call before resolving the standard Err constructor.
    let ExprKind::Call(callee, [error]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };

    // Match both the function's error type and the resolved standard constructor.
    if result_error_ty(cx, cx.typeck_results().expr_ty(expr)) != Some(function_error_ty)
        || !resolved_std_err(cx, qpath, callee.hir_id)
    {
        return None;
    }

    Some(error)
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

/// Return whether resolution found std err.
fn resolved_std_err(cx: &LateContext<'_>, qpath: QPath<'_>, hir_id: rustc_hir::HirId) -> bool {
    let Res::Def(_, def_id) = cx.qpath_res(&qpath, hir_id) else {
        return false;
    };

    let def_path = cx.tcx.def_path_str(def_id);
    // Use the resolved definition path, not the source spelling, so prelude and qualified
    // standard `Err` paths still match while local functions named `Err` are rejected.
    def_path == "std::prelude::v1::Err" || def_path.contains("result::Result::Err")
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: Option<String>,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(message);
            if let Some(suggestion) = suggestion {
                let _ =
                    diag.span_suggestion(span, help, suggestion, Applicability::MachineApplicable);
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
