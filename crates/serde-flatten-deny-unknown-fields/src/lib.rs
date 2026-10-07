#![feature(rustc_private)]

//! A lint to check for `serde(flatten)` combined with `deny_unknown_fields`.
//!
//! This Dylint library finds flattened fields in deserializable structs when
//! the outer struct or the resolved flattened struct rejects unknown fields.
//! The flattened type is resolved through aliases, paths, and `Option`, so the
//! lint finds the exact struct rather than one that shares its name.

extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

use serde_support::{AdtKind, Help, emit_lint, has_serde_attr, serde_attr, serde_item};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_FLATTEN_DENY_UNKNOWN_FIELDS,
    Warn,
    "`serde(flatten)` is not supported with `deny_unknown_fields`",
    SerdeFlattenDenyUnknownFields
}

impl<'tcx> LateLintPass<'tcx> for SerdeFlattenDenyUnknownFields {
    /// Check the flattened fields of one deserializable struct.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve derives before checking the deserialization-only attribute.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        // `deny_unknown_fields` affects only deserialization.
        if item.kind != AdtKind::Struct || !item.derives.has_deserialize {
            return;
        }

        // Report flattening when either side rejects unknown fields.
        let outer_denies = has_serde_attr(item.attrs, "deny_unknown_fields");
        for field in &item.fields {
            let Some(flatten_attr) = serde_attr(field.attrs, "flatten") else {
                continue;
            };
            // Inspect the resolved field type, not its spelling.
            let field_ty = cx
                .tcx
                .type_of(field.def_id)
                .instantiate_identity()
                .skip_norm_wip();
            if outer_denies || is_denying_struct(cx, field_ty) {
                emit_lint(
                    cx,
                    SERDE_FLATTEN_DENY_UNKNOWN_FIELDS,
                    field.hir_id,
                    flatten_attr.span(),
                    "`serde(flatten)` is not supported with `deny_unknown_fields`",
                    Help::text(
                        "remove `deny_unknown_fields` from the outer or flattened struct, or avoid `flatten`",
                    ),
                );
            }
        }
    }
}

/// Return whether the flattened type, or the `T` of `Option<T>`, is a local struct
/// that rejects unknown fields.
fn is_denying_struct<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, arguments) = ty.kind() else {
        return false;
    };
    // Serde flattens `Option<T>` through `T`.
    if cx.tcx.is_diagnostic_item(sym::Option, adt.did()) {
        return is_denying_struct(cx, arguments.type_at(0));
    }
    adt.is_struct()
        && adt.did().as_local().is_some_and(|local| {
            has_serde_attr(
                cx.tcx.hir_attrs(cx.tcx.local_def_id_to_hir_id(local)),
                "deny_unknown_fields",
            )
        })
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
