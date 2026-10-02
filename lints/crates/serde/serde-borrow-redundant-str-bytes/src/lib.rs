#![feature(rustc_private)]

//! A lint to check for redundant serde borrow attributes on borrowed strings and bytes.
//!
//! This Dylint library finds `#[serde(borrow)]` on fields whose written type
//! `serde_derive` already borrows implicitly, and removes the attribute when it
//! holds no other entry. Type aliases are not flagged, because Serde does
//! not borrow through them.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{
    Help, all_fields, emit_lint, is_deletable_attr, serde_attr, serde_item,
    ty_is_implicitly_borrowed,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_BORROW_REDUNDANT_STR_BYTES,
    Warn,
    "`serde(borrow)` is redundant on `&str` and `&[u8]` fields",
    SerdeBorrowRedundantStrBytes
}

impl<'tcx> LateLintPass<'tcx> for SerdeBorrowRedundantStrBytes {
    /// Check the fields of one deserializable item.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only deserializable items borrow from their input.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if !item.derives.has_deserialize {
            return;
        }

        // Report attributes only when Serde already borrows the written field type.
        for field in all_fields(&item) {
            let Some(borrow_attr) = serde_attr(field.attrs, "borrow") else {
                continue;
            };
            if !ty_is_implicitly_borrowed(field.ty) {
                continue;
            }

            // Deleting the attribute is exact only when `borrow` is its sole entry.
            let help = if is_deletable_attr(cx, borrow_attr, "borrow") {
                "remove the redundant borrow attribute"
            } else {
                "remove `borrow` from this attribute"
            };
            emit_lint(
                cx,
                SERDE_BORROW_REDUNDANT_STR_BYTES,
                field.hir_id,
                borrow_attr.span(),
                "`serde(borrow)` is redundant on this field type",
                Help::attr_deletion(cx, borrow_attr, "borrow", help),
            );
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
