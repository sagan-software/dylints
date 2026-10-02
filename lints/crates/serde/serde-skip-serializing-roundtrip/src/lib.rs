#![feature(rustc_private)]

//! A lint to check for serde `skip_serializing` fields that still deserialize.
//!
//! This Dylint library resolves Serde field attributes, reports asymmetric
//! serialization policy, and recommends matching serialization and deserialization.
//!
//! The README defines the supported field attributes and replacement. UI
//! fixtures cover triggering and non-triggering forms for safe adoption.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ast_all_fields, ast_has_serde_attr, ast_serde_attr, emit_span_lint_with_help, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_SKIP_SERIALIZING_ROUNDTRIP,
    Warn,
    "`serde(skip_serializing)` field is still required when deserializing",
    SerdeSkipSerializingRoundtrip
}

impl EarlyLintPass for SerdeSkipSerializingRoundtrip {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check serializable/deserializable fields for one-way skip attributes.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde items and resolved type facts once for the crate.
    let krate = serde_ast_crate(cx, krate);

    // Restrict round-trip checks to items that support both directions.
    for item in &krate.items {
        if !(item.derives.has_serialize && item.derives.has_deserialize) {
            continue;
        }
        let container_default = ast_has_serde_attr(cx, item.attrs, "default");

        // Report one-way field skips only when deserialization cannot fill the field.
        for field in ast_all_fields(item) {
            let Some(skip_serializing) = ast_serde_attr(cx, field.attrs, "skip_serializing") else {
                continue;
            };
            if field_deserializes_safely(cx, field.attrs, container_default) {
                continue;
            }

            emit_span_lint_with_help(
                cx,
                SERDE_SKIP_SERIALIZING_ROUNDTRIP,
                skip_serializing.span,
                "`skip_serializing` does not skip deserializing this field",
                "use `skip`, add `skip_deserializing`, or provide a default for deserialization",
            );
        }
    }
}

/// Return whether serde can fill a skipped field during deserialization.
fn field_deserializes_safely(
    cx: &EarlyContext<'_>,
    attrs: &[rustc_ast::Attribute],
    container_default: bool,
) -> bool {
    // Serde fills skipped/defaulted fields during deserialization, so round-trips can succeed.
    container_default
        || ast_has_serde_attr(cx, attrs, "skip")
        || ast_has_serde_attr(cx, attrs, "skip_deserializing")
        || ast_has_serde_attr(cx, attrs, "default")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
