#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only integer field types are relevant and diagnostics are configured in place"
)]

//! A lint to check for timestamp fields stored as integers.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

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
    pub DATETIME_INTEGER_FIELD,
    Warn,
    "timestamp-like field stored as an integer",
    DatetimeIntegerField
}

impl<'tcx> LateLintPass<'tcx> for DatetimeIntegerField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Resolve the field type only when its name denotes temporal data.
        let field_name = field.ident.name.to_ident_string();

        // Report raw integers at the field type span with a typed replacement.
        if datetime_field_name(&field_name)
            && let Some(integer_ty) = integer_ty(
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            emit_span_lint_with_help(
                cx,
                DATETIME_INTEGER_FIELD,
                field.ty.span,
                format!("timestamp-like field `{field_name}` uses integer `{integer_ty}`"),
                "use `chrono::DateTime`, `chrono::NaiveDate`, or another semantic datetime type",
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

    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return whether a field name denotes a timestamp or calendar date.
fn datetime_field_name(name: &str) -> bool {
    let tokens = name.split('_').collect::<Vec<_>>();

    // Match direct timestamp tokens and common `created_at_ts` style spellings.
    // A bare `unix` also names permissions or modes, so it needs a time word beside it.
    tokens.iter().any(|token| {
        matches!(
            *token,
            "timestamp" | "timestamps" | "epoch" | "ts" | "date" | "datetime"
        )
    }) || tokens.windows(2).any(|window| {
        matches!(
            window,
            ["at", "ts" | "epoch" | "unix"]
                | [
                    "unix",
                    "time" | "secs" | "seconds" | "ms" | "millis" | "nanos"
                ]
        )
    })
}

/// Return the written name of a resolved primitive integer type.
fn integer_ty(ty: ty::Ty<'_>) -> Option<&'static str> {
    // Resolve aliases before matching so only raw integer storage warns.
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
