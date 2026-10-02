#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for incompatible clap multicall and no-binary-name settings.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use clap as _;

use clap_support::{BuilderType, bool_argument, builder_calls, is_outermost_builder_call};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_MULTICALL_NO_BINARY_NAME,
    Warn,
    "`clap::Command` enables incompatible command-name parsing modes",
    ClapMulticallNoBinaryName
}

impl<'tcx> LateLintPass<'tcx> for ClapMulticallNoBinaryName {
    /// Check one expression for a complete clap command builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }

        // Read calls in source order so later boolean settings replace earlier ones.
        let mut multicall = None;
        let mut no_binary_name = None;
        for call in builder_calls(cx, expr, BuilderType::Command) {
            match call.method.as_str() {
                "multicall" => {
                    if let Some(value) = bool_argument(call) {
                        multicall = Some((value, call.span));
                    }
                }
                "no_binary_name" => {
                    if let Some(value) = bool_argument(call) {
                        no_binary_name = Some((value, call.span));
                    }
                }
                _ => {}
            }
        }

        // Require both final literal settings to remain enabled.
        let (Some((true, _)), Some((true, span))) = (multicall, no_binary_name) else {
            return;
        };
        emit_span_lint_with_help(
            cx,
            CLAP_MULTICALL_NO_BINARY_NAME,
            span,
            "`multicall(true)` cannot be combined with `no_binary_name(true)`",
            "remove `no_binary_name(true)`; multicall mode already controls binary-name parsing",
        );
    }
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
