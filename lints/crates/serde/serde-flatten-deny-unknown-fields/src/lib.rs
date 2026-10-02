#![feature(rustc_private)]

//! A lint to check for unsupported serde flatten and `deny_unknown_fields` combinations.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use std::collections::BTreeSet;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    ItemKind, ast_has_serde_attr, ast_serde_attr, ast_ty_resolves_to_any_name,
    emit_span_lint_with_help, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_FLATTEN_DENY_UNKNOWN_FIELDS,
    Warn,
    "`serde(flatten)` is not supported with `deny_unknown_fields`",
    SerdeFlattenDenyUnknownFields
}

impl EarlyLintPass for SerdeFlattenDenyUnknownFields {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check serde structs for `flatten` combined with denied unknown fields.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active items and aliases before resolving flattened field types.
    let krate = serde_ast_crate(cx, krate);
    // Build the denied inner-struct set once for all outer fields.
    let denied_structs = denied_struct_names(cx, &krate.items);

    // Report flattening when either side rejects unknown fields.
    for item in &krate.items {
        if item.kind != ItemKind::Struct || !item.derives.has_serde() {
            continue;
        }

        let outer_denies = ast_has_serde_attr(cx, item.attrs, "deny_unknown_fields");
        for field in &item.fields {
            let Some(flatten_attr) = ast_serde_attr(cx, field.attrs, "flatten") else {
                continue;
            };
            if outer_denies
                || ast_ty_resolves_to_any_name(field.ty, &krate.type_facts, &denied_structs)
            {
                emit_span_lint_with_help(
                    cx,
                    SERDE_FLATTEN_DENY_UNKNOWN_FIELDS,
                    flatten_attr.span,
                    "`serde(flatten)` is not supported with `deny_unknown_fields`",
                    "remove `deny_unknown_fields` from the outer or flattened struct, or avoid `flatten`",
                );
            }
        }
    }
}

/// Collect struct names that carry `#[serde(deny_unknown_fields)]`.
fn denied_struct_names(
    cx: &EarlyContext<'_>,
    items: &[serde_support::AstItemInfo<'_>],
) -> BTreeSet<String> {
    items
        .iter()
        .filter(|item| item.kind == ItemKind::Struct)
        .filter(|item| ast_has_serde_attr(cx, item.attrs, "deny_unknown_fields"))
        .map(|item| item.name.clone())
        .collect()
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
