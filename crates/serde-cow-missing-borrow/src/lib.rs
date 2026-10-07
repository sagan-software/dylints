#![feature(rustc_private)]

//! A lint to check for borrowable `Cow` fields without `#[serde(borrow)]`.
//!
//! This Dylint library finds fields written as `Cow<'a, str>` or
//! `Cow<'a, [u8]>` in deserializable types. Serde deserializes those as owned
//! data unless the field opts into borrowing. The lint mirrors the written
//! type check that `serde_derive` performs, so it reports only fields that the
//! attribute would actually change.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{
    Help, all_fields, emit_lint, has_serde_attr, serde_item, ty_is_borrowable_cow,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_COW_MISSING_BORROW,
    Warn,
    "`Cow` field does not opt into serde borrowing",
    SerdeCowMissingBorrow
}

impl<'tcx> LateLintPass<'tcx> for SerdeCowMissingBorrow {
    /// Check the fields of one deserializable item.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only deserializable items can borrow from their input.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if !item.derives.has_deserialize {
            return;
        }

        // Report borrowable `Cow` fields that do not opt into borrowing.
        for field in all_fields(&item) {
            if !ty_is_borrowable_cow(cx, field.ty) || has_serde_attr(field.attrs, "borrow") {
                continue;
            }
            emit_lint(
                cx,
                SERDE_COW_MISSING_BORROW,
                field.hir_id,
                field.span,
                "`Cow` fields need `#[serde(borrow)]` to deserialize borrowed data",
                Help::text("add `#[serde(borrow)]` if this field should borrow from the input"),
            );
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
