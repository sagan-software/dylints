#![feature(rustc_private)]

//! A lint to check for Serde field defaults replaceable by a container default.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ItemKind, ast_has_serde_attr, ast_serde_attr, ast_serde_directional_value,
    emit_span_lint_with_help, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_ALL_FIELDS_DEFAULT,
    Warn,
    "repeated serde field defaults can use a container default attribute",
    SerdeAllFieldsDefault
}

impl EarlyLintPass for SerdeAllFieldsDefault {
    /// Check all cfg-active Serde structs in the crate.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check derived-default structs for bare defaults on every field.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde items before testing the shared default pattern.
    let krate = serde_ast_crate(cx, krate);

    // Keep only derived-default structs whose fields can share a container default.
    for item in &krate.items {
        if item.kind != ItemKind::Struct {
            continue;
        }
        if !item.derives.has_deserialize || !item.derives.has_default {
            continue;
        }
        if item.fields.len() < 2 || ast_has_serde_attr(cx, item.attrs, "default") {
            continue;
        }
        if !item.fields.iter().all(|field| {
            ast_has_serde_attr(cx, field.attrs, "default")
                && ast_serde_directional_value(cx, field.attrs, "default").is_none()
        }) {
            continue;
        }

        // Anchor the diagnostic at the first redundant field-level attribute.
        let Some(first_attr) = item
            .fields
            .iter()
            .find_map(|field| ast_serde_attr(cx, field.attrs, "default"))
        else {
            continue;
        };
        emit_span_lint_with_help(
            cx,
            SERDE_ALL_FIELDS_DEFAULT,
            first_attr.span,
            "all fields use `serde(default)` with a derived `Default` implementation",
            "add `#[serde(default)]` to the struct and remove its field-level defaults",
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
