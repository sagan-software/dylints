#![feature(rustc_private)]

//! A lint to check for serde Cow fields that do not opt into borrowing.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Serde field attributes, reports borrowable Cow
//! fields without borrowing, and recommends adding the required borrow marker.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ast_all_fields, ast_has_serde_attr, ast_ty_is_borrowable_cow, emit_span_lint_with_help,
    serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_COW_MISSING_BORROW,
    Warn,
    "`Cow` field does not opt into serde borrowing",
    SerdeCowMissingBorrow
}

impl EarlyLintPass for SerdeCowMissingBorrow {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check deserializable items for borrowable `Cow` fields without `#[serde(borrow)]`.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active items and alias facts before inspecting field types.
    let krate = serde_ast_crate(cx, krate);

    // Restrict checks to deserializable items with borrowable `Cow` fields.
    for item in &krate.items {
        if !item.derives.has_deserialize {
            continue;
        }

        for field in ast_all_fields(item) {
            if !ast_ty_is_borrowable_cow(field.ty, &krate.type_facts)
                || ast_has_serde_attr(cx, field.attrs, "borrow")
            {
                continue;
            }

            emit_span_lint_with_help(
                cx,
                SERDE_COW_MISSING_BORROW,
                field.span,
                "`Cow` fields need `#[serde(borrow)]` to deserialize borrowed data",
                "add `#[serde(borrow)]` if this field should borrow from the input",
            );
        }
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
