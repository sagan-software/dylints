#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for clap positional-only last settings on options.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderCall, BuilderType, bool_argument, builder_calls, is_outermost_builder_call,
};
use rustc_ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_LAST_OPTION,
    Warn,
    "`clap::Arg::last` is enabled on an option",
    ClapLastOption
}

impl<'tcx> LateLintPass<'tcx> for ClapLastOption {
    /// Check one expression for a complete clap argument builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }

        // Keep only the final literal setting for each independently resettable property.
        let mut last = None;
        let mut long = None;
        let mut short = None;
        for call in builder_calls(cx, expr, BuilderType::Arg) {
            if call.method.as_str() == "last" {
                last = bool_argument(call).map(|value| (value, call.span));
            } else if call.method.as_str() == "long" {
                long = option_name_argument(call);
            } else if call.method.as_str() == "short" {
                short = option_name_argument(call);
            }
        }

        // Require an enabled final `last` setting and at least one option name.
        let Some((true, span)) = last else {
            return;
        };
        if long != Some(true) && short != Some(true) {
            return;
        }

        // Anchor the conflict at the final enabled `last` setting.
        emit_span_lint_with_help(
            cx,
            CLAP_LAST_OPTION,
            span,
            "`last(true)` has no effect on a clap option",
            "remove `last(true)`, or make the argument positional by removing its long and short names",
        );
    }
}

/// Recognize literal option names while treating dynamic or reset values as unknown.
const fn option_name_argument(call: BuilderCall<'_>) -> Option<bool> {
    // Treat string and character literals as known option names.
    let [argument] = call.args else {
        return None;
    };
    let ExprKind::Lit(literal) = argument.kind else {
        return None;
    };

    // Only literal long or short names establish option status.
    if let LitKind::Str(..) | LitKind::Char(_) = literal.node {
        return Some(true);
    }
    None
}

/// Emit the lint with a focused remediation.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic.primary_message(message).help(help);
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
