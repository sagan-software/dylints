#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for manual command line argument parsing.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Expr, ExprKind, HirId, QPath,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ARG_PARSING,
    Warn,
    "manual command line argument parsing",
    ManualArgParsing
}

impl<'tcx> LateLintPass<'tcx> for ManualArgParsing {
    /// Check expr for this lint.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Restrict the check to resolved function calls before testing API identity.
        let ExprKind::Call(callee, _) = expr.kind else {
            return;
        };

        if !resolved_env_args_call(cx, callee) {
            return;
        }

        emit_span_lint_with_help(
            cx,
            MANUAL_ARG_PARSING,
            callee.span,
            "command line arguments are parsed manually",
            "derive `clap::Parser` for the CLI shape, or use `clap` parsing instead",
        );
    }
}

/// Return whether resolution found env args call.
fn resolved_env_args_call(cx: &LateContext<'_>, callee: &Expr<'_>) -> bool {
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };

    // Use rustc resolution instead of name-only matching so local `args()` helpers are ignored.
    resolves_to_std_env_args(cx, qpath, callee.hir_id)
}

/// Helper for resolves to std env args analysis.
fn resolves_to_std_env_args(cx: &LateContext<'_>, qpath: QPath<'_>, hir_id: HirId) -> bool {
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(&qpath, hir_id) else {
        return false;
    };

    // The resolved definition catches fully qualified calls, module aliases, and imported
    // functions while still rejecting unrelated local functions named `args`.
    matches!(
        cx.tcx.def_path_str(def_id).as_str(),
        "std::env::args" | "std::env::args_os"
    )
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
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
