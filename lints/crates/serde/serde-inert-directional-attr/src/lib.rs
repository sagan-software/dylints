#![feature(rustc_private)]

//! A lint to check for serde attributes that cannot affect the derived direction.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Serde derive directions and attributes, reports
//! inert settings, and recommends removing or relocating the unused marker.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_errors::Applicability;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{
    AstItemInfo, ast_all_fields, ast_attr_has_word, ast_attr_is_single_entry,
    emit_span_lint_with_help, emit_span_lint_with_suggestion, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_INERT_DIRECTIONAL_ATTR,
    Warn,
    "`serde` attribute cannot affect the only derived direction",
    SerdeInertDirectionalAttr
}

impl EarlyLintPass for SerdeInertDirectionalAttr {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check crate for this lint.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde items before comparing derive directions and attributes.
    let krate = serde_ast_crate(cx, krate);

    // Inspect directional attributes only when the item derives one direction.
    for item in &krate.items {
        if !(item.derives.only_serialize() || item.derives.only_deserialize()) {
            continue;
        }

        for attr in directional_attrs(item) {
            let Some(inert_key) = inert_key_for_item(cx, item, attr) else {
                continue;
            };
            // Use a machine-applicable deletion only when the attribute is one simple entry.
            if ast_attr_is_single_entry(cx, attr, inert_key) {
                emit_span_lint_with_suggestion(
                    cx,
                    SERDE_INERT_DIRECTIONAL_ATTR,
                    attr.span,
                    "`serde` attribute is inert for the only derived direction",
                    inert_help(inert_key, item),
                    String::new(),
                    Applicability::MachineApplicable,
                );
            } else {
                emit_span_lint_with_help(
                    cx,
                    SERDE_INERT_DIRECTIONAL_ATTR,
                    attr.span,
                    "`serde` attribute is inert for the only derived direction",
                    inert_help(inert_key, item),
                );
            }
        }
    }
}

/// Helper for directional attrs analysis.
fn directional_attrs<'item>(item: &'item AstItemInfo<'_>) -> Vec<&'item rustc_ast::Attribute> {
    // Collect container attributes before nested field and variant attributes.
    let mut attrs = Vec::new();
    attrs.extend(
        item.attrs
            .iter()
            .filter(|attr| attr.has_name(rustc_span::Symbol::intern("serde"))),
    );
    for field in ast_all_fields(item) {
        attrs.extend(
            field
                .attrs
                .iter()
                .filter(|attr| attr.has_name(rustc_span::Symbol::intern("serde"))),
        );
    }
    // Variant attributes are distinct from their fields already returned above.
    for variant in &item.variants {
        attrs.extend(
            variant
                .attrs
                .iter()
                .filter(|attr| attr.has_name(rustc_span::Symbol::intern("serde"))),
        );
    }

    attrs
}

/// Helper for inert key for item analysis.
fn inert_key_for_item<'key>(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    attr: &rustc_ast::Attribute,
) -> Option<&'key str> {
    // Select the keys that affect only the direction absent from this item.
    if item.derives.only_serialize() {
        return first_matching_key(cx, attr, DESERIALIZE_ONLY_KEYS);
    }
    if item.derives.only_deserialize() {
        return first_matching_key(cx, attr, SERIALIZE_ONLY_KEYS);
    }

    None
}

/// Return the first matching key.
fn first_matching_key<'key>(
    cx: &EarlyContext<'_>,
    attr: &rustc_ast::Attribute,
    keys: &[&'key str],
) -> Option<&'key str> {
    keys.iter()
        .copied()
        .find(|key| ast_attr_has_word(cx, attr, key))
}

/// Helper for inert help analysis.
fn inert_help(key: &str, item: &AstItemInfo<'_>) -> &'static str {
    if item.derives.only_serialize() && DESERIALIZE_ONLY_KEYS.contains(&key) {
        "remove the deserialization-only attribute, or add `Deserialize` if it was intended"
    } else {
        "remove the serialization-only attribute, or add `Serialize` if it was intended"
    }
}

/// `DESERIALIZE_ONLY_KEYS` configuration used by this lint.
const DESERIALIZE_ONLY_KEYS: &[&str] = &[
    "alias",
    "default",
    "deserialize_with",
    "borrow",
    "skip_deserializing",
];
/// `SERIALIZE_ONLY_KEYS` configuration used by this lint.
const SERIALIZE_ONLY_KEYS: &[&str] = &[
    "skip_serializing",
    "skip_serializing_if",
    "serialize_with",
    "getter",
];

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
