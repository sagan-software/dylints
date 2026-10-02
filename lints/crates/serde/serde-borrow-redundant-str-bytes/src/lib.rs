#![feature(rustc_private)]

//! A lint to check for redundant serde borrow attributes on borrowed strings and bytes.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ast_all_fields, ast_attr_is_single_entry, ast_serde_attr, ast_ty_is_implicitly_borrowed,
    emit_span_lint_with_help, emit_span_lint_with_suggestion, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_BORROW_REDUNDANT_STR_BYTES,
    Warn,
    "`serde(borrow)` is redundant on `&str` and `&[u8]` fields",
    SerdeBorrowRedundantStrBytes
}

impl EarlyLintPass for SerdeBorrowRedundantStrBytes {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check deserializable items for redundant borrow attributes on already-borrowed fields.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active items and alias facts before inspecting field types.
    let krate = serde_ast_crate(cx, krate);

    // Restrict checks to deserializable items with explicit borrow attributes.
    for item in &krate.items {
        if !item.derives.has_deserialize {
            continue;
        }

        // Report attributes only when the field type already borrows implicitly.
        for field in ast_all_fields(item) {
            let Some(borrow_attr) = ast_serde_attr(cx, field.attrs, "borrow") else {
                continue;
            };
            if !ast_ty_is_implicitly_borrowed(field.ty, &krate.type_facts) {
                continue;
            }

            let message = "`serde(borrow)` is redundant on this field type";
            // Deleting the attribute is exact only when `borrow` is its sole entry.
            if ast_attr_is_single_entry(cx, borrow_attr, "borrow") {
                emit_span_lint_with_suggestion(
                    cx,
                    SERDE_BORROW_REDUNDANT_STR_BYTES,
                    borrow_attr.span,
                    message,
                    "remove the redundant borrow attribute",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            } else {
                emit_span_lint_with_help(
                    cx,
                    SERDE_BORROW_REDUNDANT_STR_BYTES,
                    borrow_attr.span,
                    message,
                    "remove `borrow` from this attribute",
                );
            }
        }
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
