#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for command-with-arguments value hints on clap options.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderCall, BuilderType, builder_calls, is_outermost_builder_call, is_value_hint_variant,
};
use rustc_ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_COMMAND_WITH_ARGUMENTS_OPTION,
    Warn,
    "`clap::ValueHint::CommandWithArguments` is used on an option",
    ClapCommandWithArgumentsOption
}

impl<'tcx> LateLintPass<'tcx> for ClapCommandWithArgumentsOption {
    /// Check one expression for a complete clap argument builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }

        // Retain the final setting for properties that later calls can replace.
        let mut command_with_arguments = None;
        let mut long = None;
        let mut short = None;
        for call in builder_calls(cx, expr, BuilderType::Arg) {
            if call.method.as_str() == "value_hint" {
                command_with_arguments = value_hint(cx, call).map(|enabled| (enabled, call.span));
            } else if call.method.as_str() == "long" {
                long = option_name_argument(call);
            } else if call.method.as_str() == "short" {
                short = option_name_argument(call);
            }
        }

        // Require the command hint and at least one option name to remain active.
        let Some((true, span)) = command_with_arguments else {
            return;
        };
        if long != Some(true) && short != Some(true) {
            return;
        }

        // Report the active hint and explain both valid alternatives.
        emit_span_lint_with_help(
            cx,
            CLAP_COMMAND_WITH_ARGUMENTS_OPTION,
            span,
            "`ValueHint::CommandWithArguments` is only valid on a positional argument",
            "remove the long and short names, or use a value hint that is valid for options",
        );
    }
}

/// Recognize the exact resolved clap value-hint variant.
fn value_hint(cx: &LateContext<'_>, call: BuilderCall<'_>) -> Option<bool> {
    let [argument] = call.args else {
        return None;
    };

    Some(is_value_hint_variant(cx, argument, "CommandWithArguments"))
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
