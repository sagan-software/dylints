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
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FieldDef, Item, ItemKind, LangItem, Variant, VariantData};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
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
        // Generated enums cannot be edited at the reported field.
        if item.span.from_expansion() {
            return;
        }
        let enum_name = enum_name.name.to_ident_string();
        if !(enum_name.ends_with("Error") || enum_name.ends_with("Errors")) {
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
fn check_variant<'tcx>(cx: &LateContext<'tcx>, variant: &Variant<'tcx>) {
    // Dispatch by variant field shape because names carry different semantics.
    match variant.data {
        VariantData::Struct { fields, .. } => check_struct_variant(cx, variant, fields),
        VariantData::Tuple(fields, ..) => check_tuple_variant(cx, variant, fields),
        VariantData::Unit(..) => {}
    }
}

/// Check struct variant for this lint.
fn check_struct_variant<'tcx>(
    cx: &LateContext<'tcx>,
    variant: &Variant<'tcx>,
    fields: &[FieldDef<'tcx>],
) {
    // Check named fields whose vocabulary implies an unrestricted message payload.
    for field in fields {
        let field_name = field.ident.name.to_ident_string();
        if broad_payload_name(&field_name)
            && let Some(string_ty) = string_ty(cx, field)
        {
            emit_broad_string_lint(cx, field.ty.span, "field", &field_name, string_ty, variant);
        }
    }
}

/// Check tuple variant for this lint.
fn check_tuple_variant<'tcx>(
    cx: &LateContext<'tcx>,
    variant: &Variant<'tcx>,
    fields: &[FieldDef<'tcx>],
) {
    // A single tuple payload with a broad variant name makes the string the contract.
    let [field] = fields else {
        return;
    };
    let variant_name = variant.ident.name.to_ident_string();
    if !broad_payload_variant_name(&variant_name) {
        return;
    }

    // Report only when that sole payload resolves to a string type.
    if let Some(string_ty) = string_ty(cx, field) {
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

/// Return the broad payload name.
fn broad_payload_name(name: &str) -> bool {
    matches!(name, "message" | "details" | "reason" | "error")
}

/// Return the broad payload variant name.
fn broad_payload_variant_name(name: &str) -> bool {
    matches!(name, "Message" | "Details" | "Reason" | "Error")
}

/// Return the display name of a field type that resolves to `String` or `&str`.
fn string_ty<'tcx>(cx: &LateContext<'tcx>, field: &FieldDef<'tcx>) -> Option<&'static str> {
    // Resolve aliases and paths through the field's semantic type.
    let ty = cx
        .tcx
        .type_of(field.def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let ty = dylint_support::peel_standard_options(cx.tcx, ty)?;

    match ty.kind() {
        ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String) => Some("String"),
        ty::Ref(_, inner, _) if inner.is_str() => Some("&str"),
        _ => None,
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
