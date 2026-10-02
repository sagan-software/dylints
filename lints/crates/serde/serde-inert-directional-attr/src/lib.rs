#![feature(rustc_private)]

//! A lint to check for serde attributes that cannot affect the derived direction.
//!
//! This Dylint library resolves which Serde traits a type derives and reports
//! container, field, and variant entries that affect only the direction the
//! type does not derive. It removes an attribute whose only entry is inert.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::{Attribute, HirId, Item};
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{
    Help, SerdeItem, all_fields, attr_has_entry, emit_lint, namespace_attrs, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_INERT_DIRECTIONAL_ATTR,
    Warn,
    "`serde` attribute cannot affect the only derived direction",
    SerdeInertDirectionalAttr
}

impl<'tcx> LateLintPass<'tcx> for SerdeInertDirectionalAttr {
    /// Check one item that derives exactly one Serde direction.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        // Select the keys that affect only the direction absent from this item.
        let (keys, help) = if item.derives.only_serialize() {
            (
                DESERIALIZE_ONLY_KEYS,
                "remove the deserialization-only attribute, or add `Deserialize` if it was intended",
            )
        } else if item.derives.only_deserialize() {
            (
                SERIALIZE_ONLY_KEYS,
                "remove the serialization-only attribute, or add `Serialize` if it was intended",
            )
        } else {
            return;
        };

        for (hir_id, attr) in directional_attrs(cx, &item) {
            let Some(key) = keys.iter().find(|key| attr_has_entry(attr, key)) else {
                continue;
            };
            // Use a machine-applicable deletion only when the attribute is one simple entry.
            emit_lint(
                cx,
                SERDE_INERT_DIRECTIONAL_ATTR,
                hir_id,
                attr.span(),
                "`serde` attribute is inert for the only derived direction",
                Help::attr_deletion(cx, attr, key, help),
            );
        }
    }
}

/// Return container, field, and variant attributes with their owning node.
fn directional_attrs<'tcx>(
    cx: &LateContext<'tcx>,
    item: &SerdeItem<'tcx>,
) -> Vec<(HirId, &'tcx Attribute)> {
    // Container attributes come first, so diagnostics follow source order.
    let item_hir_id = cx.tcx.local_def_id_to_hir_id(item.def_id);
    let mut attrs: Vec<_> = namespace_attrs(item.attrs, "serde")
        .map(|attr| (item_hir_id, attr))
        .collect();
    // Field attributes carry the field node so `allow` on a field applies.
    for field in all_fields(item) {
        attrs.extend(namespace_attrs(field.attrs, "serde").map(|attr| (field.hir_id, attr)));
    }
    for variant in &item.variants {
        attrs.extend(namespace_attrs(variant.attrs, "serde").map(|attr| (variant.hir_id, attr)));
    }
    attrs
}

/// Serde keys that affect only deserialization.
const DESERIALIZE_ONLY_KEYS: &[&str] = &[
    "alias",
    "default",
    "deserialize_with",
    "borrow",
    "skip_deserializing",
];
/// Serde keys that affect only serialization.
const SERIALIZE_ONLY_KEYS: &[&str] = &[
    "skip_serializing",
    "skip_serializing_if",
    "serialize_with",
    "getter",
];

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
