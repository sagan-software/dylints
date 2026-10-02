#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]

//! A lint to check for country-code-like fields stored as strings.
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
use rustc_hir::{FieldDef, LangItem};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub COUNTRY_STRING_FIELD,
    Warn,
    "country-code-like field stored as a string",
    CountryStringField
}

impl<'tcx> LateLintPass<'tcx> for CountryStringField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Read the API name before resolving the field's semantic type.
        let field_name = field.ident.name.to_ident_string();

        // Diagnose only country vocabulary paired with a resolved string representation.
        if country_field_name(&field_name)
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
                COUNTRY_STRING_FIELD,
                field.ty.span,
                format!("country-code-like field `{field_name}` uses `{string_ty}`"),
                "use a validated country-code type or project-specific country newtype",
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

    // Use rustc's native diagnostic decorator to match the surrounding type lints.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return the country field name.
fn country_field_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();

    // Keep the vocabulary focused on country-code fields instead of broad display-name strings.
    matches!(
        lower.as_str(),
        "country"
            | "country_code"
            | "iso_country"
            | "iso_country_code"
            | "residence_country"
            | "nationality_country"
    ) || lower.ends_with("_country")
        || lower.ends_with("_country_code")
}

/// Return type information for string.
fn string_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> Option<&'static str> {
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
