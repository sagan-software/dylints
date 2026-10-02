#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for command-with-arguments value hints on clap options.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderType, builder_calls, clap_enum_variant, emit_lint_with_help, has_option_name,
    is_outermost_builder_call,
};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};

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
        let calls = builder_calls(cx, expr, BuilderType::Arg);

        // The final value hint replaces earlier ones.
        let Some(hint) = calls
            .iter()
            .rev()
            .find(|call| call.method.as_str() == "value_hint")
        else {
            return;
        };
        let is_command_hint = matches!(
            hint.args,
            [argument] if clap_enum_variant(cx, argument, "ValueHint")
                .is_some_and(|variant| variant.as_str() == "CommandWithArguments")
        );
        // Require the command hint and at least one option name to remain active.
        if !is_command_hint || !has_option_name(cx, &calls) {
            return;
        }

        // Report the active hint and explain both valid alternatives.
        emit_lint_with_help(
            cx,
            CLAP_COMMAND_WITH_ARGUMENTS_OPTION,
            hint.span,
            "`ValueHint::CommandWithArguments` is only valid on a positional argument",
            "remove the long and short names, or use a value hint that is valid for options",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
