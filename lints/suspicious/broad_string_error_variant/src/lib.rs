#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "the lint targets selected error field types and configures diagnostics in place"
)]

//! A lint to check for broad string payloads on error enum variants.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FieldDef, Item, ItemKind, QPath, Ty, TyKind, Variant, VariantData};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BROAD_STRING_ERROR_VARIANT,
    Warn,
    "error enum variant carries a broad string payload",
    BroadStringErrorVariant
}

impl<'tcx> LateLintPass<'tcx> for BroadStringErrorVariant {
    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let ItemKind::Enum(enum_name, _generics, enum_def) = item.kind else {
            return;
        };
        let enum_name = enum_name.name.to_ident_string();
        if !error_like_enum_name(&enum_name) {
            return;
        }

        // Only enum variants know whether their fields are named or positional.
        // Check each variant under the established error-enum boundary.
        for variant in enum_def.variants {
            check_variant(cx, variant);
        }
    }
}

/// Check variant for this lint.
fn check_variant(cx: &LateContext<'_>, variant: &Variant<'_>) {
    // Dispatch by variant field shape because names carry different semantics.
    match variant.data {
        VariantData::Struct { fields, .. } => check_struct_variant(cx, variant, fields),
        VariantData::Tuple(fields, ..) => check_tuple_variant(cx, variant, fields),
        VariantData::Unit(..) => {}
    }
}

/// Check struct variant for this lint.
fn check_struct_variant(cx: &LateContext<'_>, variant: &Variant<'_>, fields: &[FieldDef<'_>]) {
    // Check named fields whose vocabulary implies an unrestricted message payload.
    for field in fields {
        let field_name = field.ident.name.to_ident_string();
        if broad_payload_name(&field_name)
            && let Some(string_ty) = string_ty(field.ty)
        {
            emit_broad_string_lint(cx, field.ty.span, "field", &field_name, string_ty, variant);
        }
    }
}

/// Check tuple variant for this lint.
fn check_tuple_variant(cx: &LateContext<'_>, variant: &Variant<'_>, fields: &[FieldDef<'_>]) {
    if fields.len() != 1 {
        return;
    }
    let variant_name = variant.ident.name.to_ident_string();
    if !broad_payload_variant_name(&variant_name) {
        return;
    }

    // A single tuple payload with a broad variant name makes the string the contract.
    let Some(field) = fields.first() else {
        return;
    };
    // Report only when that sole payload resolves to a string type.
    if let Some(string_ty) = string_ty(field.ty) {
        emit_broad_string_lint(
            cx,
            field.ty.span,
            "tuple variant",
            &variant_name,
            string_ty,
            variant,
        );
    }
}

/// Emit the broad string lint diagnostic.
fn emit_broad_string_lint(
    cx: &LateContext<'_>,
    span: Span,
    label: &'static str,
    name: &str,
    string_ty: &'static str,
    variant: &Variant<'_>,
) {
    // Build a message that distinguishes named fields from tuple payloads.
    // Recommend a typed source, variant, or redacted domain value.
    emit_span_lint_with_help(
        cx,
        BROAD_STRING_ERROR_VARIANT,
        span,
        format!(
            "error variant `{}` has broad {label} `{name}` using `{string_ty}`",
            variant.ident
        ),
        "use a concrete source error, a focused variant, or a redacted domain value",
    );
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

    // Use rustc's native diagnostic decorator to match the nearby error lints.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return the error like enum name.
fn error_like_enum_name(name: &str) -> bool {
    name.ends_with("Error") || name.ends_with("Errors")
}

/// Return the broad payload name.
fn broad_payload_name(name: &str) -> bool {
    matches!(name, "message" | "details" | "reason" | "error")
}

/// Return the broad payload variant name.
fn broad_payload_variant_name(name: &str) -> bool {
    matches!(name, "Message" | "Details" | "Reason" | "Error")
}

/// Return type information for string.
fn string_ty(ty: &Ty<'_>) -> Option<&'static str> {
    match ty.kind {
        TyKind::Path(QPath::Resolved(_, path)) => {
            let segment_name = path.segments.last()?.ident.name.to_ident_string();
            (segment_name == "String").then_some("String")
        }
        TyKind::Ref(_, mut_ty) => str_ty(mut_ty.ty).then_some("&str"),
        _ => None,
    }
}

/// Return type information for str.
fn str_ty(ty: &Ty<'_>) -> bool {
    // Keep aliases out of scope so the diagnostic is tied to source-written broad strings.
    let TyKind::Path(QPath::Resolved(_, path)) = ty.kind else {
        return false;
    };

    path.segments
        .last()
        .is_some_and(|segment| segment.ident.name.to_ident_string() == "str")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
