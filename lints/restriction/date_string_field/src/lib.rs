#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only date string fields are relevant and diagnostics are configured in place"
)]

//! A lint to check for date-like fields stored as strings.
//!
//! It recognizes field names that describe dates or timestamps, resolves their
//! underlying string type through transparent wrappers, and reports raw text at
//! the storage boundary. The recommendation keeps lexical parsing at ingress
//! while retaining a typed temporal value for internal comparisons and output.

// The README doctest uses this dependency from its separate compilation unit.
#[cfg(test)]
use chrono as _;

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FieldDef, LangItem};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub DATE_STRING_FIELD,
    Warn,
    "date-like field stored as a string",
    DateStringField
}

impl<'tcx> LateLintPass<'tcx> for DateStringField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Read the API name before resolving the field's semantic type.
        let field_name = field.ident.name.to_ident_string();

        // Diagnose only date vocabulary paired with a resolved string representation.
        if date_field_name(&field_name)
            && let Some(string_ty) = string_ty(
                cx,
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            emit_span_lint_with_help(
                cx,
                DATE_STRING_FIELD,
                field.ty.span,
                format!("date-like field `{field_name}` uses `{string_ty}`"),
                "use `chrono::NaiveDate`, `time::Date`, or a validated date newtype",
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

    // Use rustc's native diagnostic decorator to keep this lint aligned with nearby type lints.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return the date field name.
fn date_field_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let tokens = lower.split('_').collect::<Vec<_>>();

    // Match exact DOB idioms, embedded `date_of_birth`, and the common semantic `_date` suffix.
    lower == "birthdate"
        || tokens.contains(&"dob")
        || tokens.windows(2).any(|window| window == ["birth", "date"])
        || tokens
            .windows(3)
            .any(|window| window == ["date", "of", "birth"])
        || lower.ends_with("_date")
}

/// Return type information for string.
fn string_ty<'tcx>(cx: &LateContext<'tcx>, ty: ty::Ty<'tcx>) -> Option<&'static str> {
    let ty = dylint_support::peel_standard_options(cx.tcx, ty)?;

    match ty.kind() {
        ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String) => Some("String"),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str) => Some("&str"),
        _ => None,
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
