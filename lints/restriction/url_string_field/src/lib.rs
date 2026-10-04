#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]

//! A lint to check for URL fields stored as strings.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

// The README doctest uses this dependency from its separate compilation unit.
#[cfg(test)]
use url as _;

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
    pub URL_STRING_FIELD,
    Warn,
    "URL-like field stored as a string",
    UrlStringField
}

impl<'tcx> LateLintPass<'tcx> for UrlStringField {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Read the API name before resolving the field's semantic type.
        let field_name = field.ident.name.to_ident_string();

        // Diagnose only URL vocabulary paired with a resolved string representation.
        if url_field_name(&field_name)
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
                URL_STRING_FIELD,
                field.ty.span,
                format!("URL-like field `{field_name}` uses `{string_ty}`"),
                "use `url::Url`, `reqwest::Url`, or a validated URL newtype",
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

/// Return the url field name.
fn url_field_name(name: &str) -> bool {
    // Match common snake_case URL field tokens without requiring project-specific naming rules.
    let has_url_token = name.split('_').any(|token| {
        matches!(
            token,
            "url" | "uri" | "endpoint" | "endpoints" | "webhook" | "link"
        )
    });

    // A final word such as `label` or `template` names text about a URL, not the URL itself.
    let is_url_description = name.rsplit('_').next().is_some_and(|token| {
        matches!(
            token,
            "description"
                | "format"
                | "label"
                | "name"
                | "pattern"
                | "prefix"
                | "suffix"
                | "template"
                | "text"
                | "title"
        )
    });
    has_url_token && !is_url_description
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
