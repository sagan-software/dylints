#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc type variants"
)]
#![warn(unused_extern_crates)]

//! A lint to check for integer measurement fields with ambiguous units.
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
use rustc_hir::{Attribute, FieldDef};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::symbol::{Symbol, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AMBIGUOUS_NUMERIC_UNIT_FIELD,
    Warn,
    "integer measurement field does not identify its unit",
    AmbiguousNumericUnitField
}

impl<'tcx> LateLintPass<'tcx> for AmbiguousNumericUnitField {
    /// Check primitive integer measurement fields for a unit-bearing boundary.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Ignore generated fields and names that already identify their unit.
        if field.span.from_expansion() {
            return;
        }

        let field_name = field.ident.name.to_ident_string();
        if !ambiguous_measurement_name(&field_name) || has_unit_token(&field_name) {
            return;
        }

        // Require a primitive integer and allow documentation to state the unit.
        let Some(integer_ty) = integer_ty(
            cx.tcx
                .type_of(field.def_id)
                .instantiate_identity()
                .skip_norm_wip(),
        ) else {
            return;
        };
        if documentation_has_unit(cx.tcx.hir_attrs(field.hir_id)) {
            return;
        }

        cx.emit_span_lint(
            AMBIGUOUS_NUMERIC_UNIT_FIELD,
            field.ty.span,
            DiagDecorator(|diag| {
                let _ = diag.primary_message(format!(
                    "integer measurement field `{field_name}` does not identify its unit"
                ));
                let _ = diag.note(format!("the field stores a primitive `{integer_ty}`"));
                let _ = diag.help(
                    "use a unit-bearing type or state the unit in the field name or documentation",
                );
            }),
        );
    }
}

/// Return whether the final name token denotes an otherwise ambiguous measurement.
fn ambiguous_measurement_name(name: &str) -> bool {
    matches!(name.rsplit('_').next(), Some("size" | "length" | "offset"))
}

/// Return whether a field name or documentation contains a recognized unit token.
fn has_unit_token(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            matches!(
                token.to_ascii_lowercase().as_str(),
                "bit"
                    | "bits"
                    | "byte"
                    | "bytes"
                    | "kb"
                    | "kib"
                    | "mb"
                    | "mib"
                    | "gb"
                    | "gib"
                    | "tb"
                    | "tib"
                    | "char"
                    | "chars"
                    | "character"
                    | "characters"
                    | "codepoint"
                    | "codepoints"
                    | "element"
                    | "elements"
                    | "entry"
                    | "entries"
                    | "item"
                    | "items"
                    | "record"
                    | "records"
                    | "row"
                    | "rows"
                    | "column"
                    | "columns"
                    | "pixel"
                    | "pixels"
                    | "sample"
                    | "samples"
                    | "frame"
                    | "frames"
                    | "page"
                    | "pages"
                    | "word"
                    | "words"
            )
        })
}

/// Return whether normalized field documentation states a recognized unit.
fn documentation_has_unit(attrs: &[Attribute]) -> bool {
    attrs
        .iter()
        .filter_map(doc_attr_text)
        .any(|doc| has_unit_token(doc.as_str()))
}

/// Return the text carried by a normalized documentation attribute.
fn doc_attr_text(attr: &Attribute) -> Option<Symbol> {
    attr.doc_str()
        .or_else(|| attr.has_name(sym::doc).then(|| attr.value_str()).flatten())
}

/// Return the written name of a primitive integer type after alias resolution.
fn integer_ty(ty: ty::Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Uint(uint_ty) => Some(uint_ty.name_str()),
        ty::Int(int_ty) => Some(int_ty.name_str()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{ambiguous_measurement_name, has_unit_token};

    /// Accept only field names whose final token denotes a measurement.
    #[test]
    fn identifies_ambiguous_measurement_names() {
        assert!(ambiguous_measurement_name("maximum_size"));
        assert!(ambiguous_measurement_name("payload_length"));
        assert!(ambiguous_measurement_name("file_offset"));
        assert!(!ambiguous_measurement_name("size_hint"));
        assert!(!ambiguous_measurement_name("retries"));
    }

    /// Recognize unit tokens without treating adjacent nouns as units.
    #[test]
    fn identifies_unit_tokens() {
        assert!(has_unit_token("maximum_size_bytes"));
        assert!(has_unit_token("Size in KiB"));
        assert!(has_unit_token("number of characters"));
        assert!(!has_unit_token("maximum_size"));
        assert!(!has_unit_token("file_offset"));
    }
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
