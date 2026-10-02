#![feature(rustc_private)]

//! A lint to check for skipped fields that deserialization still requires.
//!
//! This Dylint library finds fields of types that derive both Serde directions
//! when serialization omits the field but deserialization has no default for
//! it, so the type's own output fails to deserialize.

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty;
use rustc_span::sym;

use serde_support::{
    Help, SerdeField, all_fields, emit_lint, has_serde_attr, serde_attr, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_SKIP_SERIALIZING_ROUNDTRIP,
    Warn,
    "`serde(skip_serializing)` field is still required when deserializing",
    SerdeSkipSerializingRoundtrip
}

impl<'tcx> LateLintPass<'tcx> for SerdeSkipSerializingRoundtrip {
    /// Check the fields of one item that derives both directions.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // A round trip needs both directions and no container default.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if !(item.derives.has_serialize && item.derives.has_deserialize)
            || has_serde_attr(item.attrs, "default")
        {
            return;
        }

        // Report one-way field skips only when deserialization cannot fill the field.
        for field in all_fields(&item) {
            let Some(skip_serializing) = serde_attr(field.attrs, "skip_serializing") else {
                continue;
            };
            if field_deserializes_when_missing(cx, field) {
                continue;
            }

            emit_lint(
                cx,
                SERDE_SKIP_SERIALIZING_ROUNDTRIP,
                field.hir_id,
                skip_serializing.span(),
                "`skip_serializing` does not skip deserializing this field",
                Help::text(
                    "use `skip`, add `skip_deserializing`, or provide a default for deserialization",
                ),
            );
        }
    }
}

/// Return whether Serde fills the field when the input omits it.
///
/// Skipped and defaulted fields get a default. A missing `Option` field becomes
/// `None`, unless a custom `deserialize_with` makes Serde report it as missing.
fn field_deserializes_when_missing(cx: &LateContext<'_>, field: &SerdeField<'_>) -> bool {
    // Skipped and defaulted fields get a default value.
    if ["skip", "skip_deserializing", "default"]
        .iter()
        .any(|key| has_serde_attr(field.attrs, key))
    {
        return true;
    }
    // A custom deserializer makes Serde report a missing field instead of `None`.
    if has_serde_attr(field.attrs, "with") || has_serde_attr(field.attrs, "deserialize_with") {
        return false;
    }
    let field_ty = cx
        .tcx
        .type_of(field.def_id)
        .instantiate_identity()
        .skip_norm_wip();
    matches!(field_ty.kind(), ty::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Option, adt.did()))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
