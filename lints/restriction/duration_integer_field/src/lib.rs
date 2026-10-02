#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc type variants"
)]

//! A lint to check for duration fields stored as integers.
//!
//! It resolves fields whose names carry a duration unit and reports primitive
//! integer storage while leaving unrelated numeric fields untouched. The help
//! text directs callers to `Duration` and keeps unit conversion at the boundary.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::FieldDef;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub DURATION_INTEGER_FIELD,
    Warn,
    "duration-like field stored as an integer",
    DurationIntegerField
}

impl<'tcx> LateLintPass<'tcx> for DurationIntegerField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Resolve the field type only when its name includes a duration unit.
        let field_name = field.ident.name.to_ident_string();

        // Report raw integers at the field type span with a typed replacement.
        if duration_unit_token(&field_name).is_some()
            && let Some(integer_ty) = integer_ty(
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            emit_span_lint_with_help(
                cx,
                DURATION_INTEGER_FIELD,
                field.ty.span,
                format!("duration-like field `{field_name}` uses integer `{integer_ty}`"),
                "use `std::time::Duration` and convert units at the input boundary",
            );
        }
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

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

/// Helper for duration unit token analysis.
fn duration_unit_token(name: &str) -> Option<&str> {
    // Match snake_case tokens so both prefix and suffix forms are covered without flagging
    // unrelated substrings such as `weekday` or `timestamp`.
    name.split('_').find(|token| {
        matches!(
            *token,
            "ns" | "nano"
                | "nanos"
                | "nanosecond"
                | "nanoseconds"
                | "us"
                | "micro"
                | "micros"
                | "microsecond"
                | "microseconds"
                | "ms"
                | "milli"
                | "millis"
                | "millisecond"
                | "milliseconds"
                | "sec"
                | "secs"
                | "second"
                | "seconds"
                | "min"
                | "mins"
                | "minute"
                | "minutes"
                | "hr"
                | "hrs"
                | "hour"
                | "hours"
                | "day"
                | "days"
                | "week"
                | "weeks"
        )
    })
}

/// Return type information for integer.
fn integer_ty(ty: ty::Ty<'_>) -> Option<&'static str> {
    // Resolve aliases before matching so the lint only targets actual primitive integer storage.
    match ty.kind() {
        ty::Uint(uint_ty) => Some(uint_ty.name_str()),
        ty::Int(int_ty) => Some(int_ty.name_str()),
        _ => None,
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
