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

        // Keep the final unconditional setting and separately prove an active condition.
        let mut required = None;
        let mut has_condition = false;
        for call in builder_calls(cx, expr, BuilderType::Arg) {
            if call.method.as_str() == "required" {
                required = bool_argument(call).map(|value| (value, call.span));
            } else if is_literal_condition(call) {
                has_condition = true;
            }
        }

        let Some((true, span)) = required else {
            return;
        };
        // A conflict exists only when both unconditional and conditional settings remain active.
        if !has_condition {
            return;
        }

        emit_span_lint_with_help(
            cx,
            CLAP_REQUIRED_CONDITIONAL_CONFLICT,
            span,
            "`required(true)` conflicts with clap's conditional requirement settings",
            "remove `required(true)` so the condition controls when the argument is required",
        );
    }
}

/// Prove that a conditional requirement call contains a nonempty literal condition.
fn is_literal_condition(call: BuilderCall<'_>) -> bool {
    // Validate arity and literal shape for each supported conditional method family.
    match call.method.as_str() {
        "required_if_eq" => {
            let [id, value] = call.args else {
                return false;
            };
            is_string_literal(id) && is_string_literal(value)
        }
        "required_unless_present" => {
            let [id] = call.args else {
                return false;
            };
            is_string_literal(id)
        }
        "required_if_eq_any"
        | "required_if_eq_all"
        | "required_unless_present_any"
        | "required_unless_present_all" => {
            // Collection forms count only when their literal array contains a condition.
            let [conditions] = call.args else {
                return false;
            };
            matches!(conditions.kind, ExprKind::Array(items) if !items.is_empty())
        }
        _ => false,
    }
}

/// Return whether an expression is a string literal accepted as a clap
/// identifier or value.
const fn is_string_literal(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal) if matches!(literal.node, LitKind::Str(..)))
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
