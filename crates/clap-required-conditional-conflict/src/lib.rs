#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for conflicting unconditional and conditional clap requirements.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Clap requirement attributes, reports conflicting
//! unconditional and conditional policies, and recommends one consistent rule.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

use clap_support::{
    BuilderCall, BuilderType, bool_argument, builder_calls, emit_lint_with_help,
    is_outermost_builder_call,
};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_REQUIRED_CONDITIONAL_CONFLICT,
    Warn,
    "`clap::Arg` is both unconditionally and conditionally required",
    ClapRequiredConditionalConflict
}

impl<'tcx> LateLintPass<'tcx> for ClapRequiredConditionalConflict {
    /// Check one expression for a complete clap argument builder chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Analyze each fluent chain once at its outermost resolved call.
        if !is_outermost_builder_call(cx, expr) {
            return;
        }
        let calls = builder_calls(cx, expr, BuilderType::Arg);

        // Keep the final unconditional setting and separately prove an active condition.
        let Some(required) = calls
            .iter()
            .rev()
            .find(|call| call.method.as_str() == "required")
        else {
            return;
        };
        // A conflict exists only when both unconditional and conditional settings remain active.
        if bool_argument(*required) != Some(true) || !calls.iter().any(|call| adds_condition(*call))
        {
            return;
        }

        emit_lint_with_help(
            cx,
            CLAP_REQUIRED_CONDITIONAL_CONFLICT,
            required.span,
            "`required(true)` conflicts with clap's conditional requirement settings",
            "remove `required(true)` so the condition controls when the argument is required",
        );
    }
}

/// Prove that a conditional requirement call adds at least one condition.
fn adds_condition(call: BuilderCall<'_>) -> bool {
    // Each method family adds conditions under a different argument shape.
    match (call.method.as_str(), call.args) {
        // The single forms always record their condition, whatever the values are.
        ("required_if_eq", [_, _]) | ("required_unless_present", [_]) => true,
        // Collection forms count only when their literal array contains a condition.
        (
            "required_if_eq_any"
            | "required_if_eq_all"
            | "required_unless_present_any"
            | "required_unless_present_all",
            [conditions],
        ) => matches!(conditions.kind, ExprKind::Array(items) if !items.is_empty()),
        _ => false,
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
