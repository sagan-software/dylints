#![feature(rustc_private)]

//! A lint to check for allocating strings passed directly to Serde `serialize_str`.
//!
//! This Dylint library resolves Serde serializer calls, reports an allocated
//! `to_string` argument, and recommends collecting the `Display` value directly.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};

use serde_support::{is_to_string_call, serde_method_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_SERIALIZE_STR_TO_STRING,
    Warn,
    "serde serialize_str receives an allocated ToString result",
    SerdeSerializeStrToString
}

impl<'tcx> LateLintPass<'tcx> for SerdeSerializeStrToString {
    /// Check semantically resolved Serde method calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact serializer method before unpacking its sole argument.
        let Some(call) = serde_method_call(cx, expr, "ser::Serializer", "serialize_str") else {
            return;
        };
        let [argument] = call.arguments else {
            return;
        };
        // Require a borrowed `to_string` result rather than another string expression.
        let Some(value) = string_value_expression(cx, argument) else {
            return;
        };
        // Both snippets come from exact method-call operands, making the local rewrite safe.
        let Some((serializer, value)) = replacement_parts(cx, call.receiver, value) else {
            return;
        };
        // Build the replacement only after both exact source snippets are available.
        let replacement = format!("{serializer}.collect_str(&{value})");

        cx.emit_span_lint(
            SERDE_SERIALIZE_STR_TO_STRING,
            expr.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("`to_string` allocates before Serde serializes the string")
                    .span_suggestion(
                        expr.span,
                        "serialize the `Display` value directly",
                        replacement,
                        Applicability::MachineApplicable,
                    );
            }),
        );
    }
}

/// Return the display operand from a borrowed `to_string` method call.
fn string_value_expression<'tcx>(
    cx: &LateContext<'tcx>,
    argument: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Inspect only the borrowed method-call shape that can be replaced safely.
    let ExprKind::AddrOf(_, _, to_string) = argument.kind else {
        return None;
    };
    let ExprKind::MethodCall(_, value, arguments, _) = to_string.kind else {
        return None;
    };
    (arguments.is_empty() && is_to_string_call(cx, to_string)).then_some(value)
}

/// Capture source snippets for a direct `collect_str` replacement.
fn replacement_parts(
    cx: &LateContext<'_>,
    receiver: &Expr<'_>,
    value: &Expr<'_>,
) -> Option<(String, String)> {
    let source_map = cx.sess().source_map();
    let serializer = source_map.span_to_snippet(receiver.span).ok()?;
    let value = source_map.span_to_snippet(value.span).ok()?;
    Some((serializer, value))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
