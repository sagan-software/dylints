#![feature(rustc_private)]

//! A lint to check for tagged Serde enum fallbacks missing `other`.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{ItemKind, ast_has_serde_attr, emit_span_lint_with_help, serde_ast_crate};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_FALLBACK_MISSING_OTHER,
    Warn,
    "tagged serde enum fallback should use serde(other)",
    SerdeFallbackMissingOther
}

impl EarlyLintPass for SerdeFallbackMissingOther {
    /// Check all cfg-active Serde enums in the crate.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check the final variant of tagged enums for a likely fallback.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde items before evaluating tagged enum policy.
    let krate = serde_ast_crate(cx, krate);

    // Keep only deserializable tagged enums whose variants lack an explicit fallback.
    for item in &krate.items {
        if !is_tagged_deserializable_enum(cx, item) {
            continue;
        }
        let Some(fallback) = fallback_variant(cx, item) else {
            continue;
        };

        emit_span_lint_with_help(
            cx,
            SERDE_FALLBACK_MISSING_OTHER,
            fallback.span,
            "this fallback-looking variant accepts only its own literal tag",
            "add `#[serde(other)]` if it should accept every unrecognized tag",
        );
    }
}

/// Return whether an enum can use a final variant as a tagged fallback.
fn is_tagged_deserializable_enum(
    cx: &EarlyContext<'_>,
    item: &serde_support::AstItemInfo<'_>,
) -> bool {
    // Require a cfg-active enum with Serde's deserialization derive.
    if item.kind != ItemKind::Enum || !item.derives.has_deserialize {
        return false;
    }
    if !ast_has_serde_attr(cx, item.attrs, "tag") || ast_has_serde_attr(cx, item.attrs, "untagged")
    {
        return false;
    }

    // An existing `other` variant already handles unknown tags.
    !item
        .variants
        .iter()
        .any(|variant| ast_has_serde_attr(cx, variant.attrs, "other"))
}

/// Return the final unit variant when it looks like an unmarked fallback.
fn fallback_variant<'ast>(
    cx: &EarlyContext<'_>,
    item: &'ast serde_support::AstItemInfo<'ast>,
) -> Option<&'ast serde_support::AstVariantInfo<'ast>> {
    // Treat only a final unit variant named Other or Unknown as a candidate.
    let fallback = item.variants.last()?;
    if !fallback.is_unit || !matches!(fallback.name.as_str(), "Other" | "Unknown") {
        return None;
    }

    // Skip variants intentionally excluded from deserialization.
    if ast_has_serde_attr(cx, fallback.attrs, "skip")
        || ast_has_serde_attr(cx, fallback.attrs, "skip_deserializing")
    {
        return None;
    }
    Some(fallback)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
