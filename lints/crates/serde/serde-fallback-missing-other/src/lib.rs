#![feature(rustc_private)]

//! A lint to check for tagged Serde enum fallbacks missing `other`.
//!
//! This Dylint library finds internally or adjacently tagged enums that derive
//! `Deserialize` and end with a unit variant named `Other` or `Unknown` that
//! lacks `#[serde(other)]`. Such a variant matches only its own tag, while
//! its name suggests it should catch every unrecognized tag.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{
    AdtKind, Help, SerdeItem, SerdeVariant, emit_lint, has_serde_attr, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_FALLBACK_MISSING_OTHER,
    Warn,
    "tagged serde enum fallback should use serde(other)",
    SerdeFallbackMissingOther
}

impl<'tcx> LateLintPass<'tcx> for SerdeFallbackMissingOther {
    /// Check the final variant of one tagged enum.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Require a tagged enum before looking at its final variant.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if !is_tagged_deserializable_enum(&item) {
            return;
        }
        let Some(fallback) = fallback_variant(&item) else {
            return;
        };

        // Report the variant itself, since the fix is an attribute on it.
        emit_lint(
            cx,
            SERDE_FALLBACK_MISSING_OTHER,
            fallback.hir_id,
            fallback.span,
            "this fallback-looking variant accepts only its own literal tag",
            Help::text("add `#[serde(other)]` if it should accept every unrecognized tag"),
        );
    }
}

/// Return whether an enum can use a final variant as a tagged fallback.
fn is_tagged_deserializable_enum(item: &SerdeItem<'_>) -> bool {
    // Require a tagged enum with Serde's deserialization derive.
    if item.kind != AdtKind::Enum || !item.derives.has_deserialize {
        return false;
    }
    if !has_serde_attr(item.attrs, "tag") || has_serde_attr(item.attrs, "untagged") {
        return false;
    }

    // An existing `other` variant already handles unknown tags.
    !item
        .variants
        .iter()
        .any(|variant| has_serde_attr(variant.attrs, "other"))
}

/// Return the final unit variant when it looks like an unmarked fallback.
fn fallback_variant<'item, 'tcx>(
    item: &'item SerdeItem<'tcx>,
) -> Option<&'item SerdeVariant<'tcx>> {
    // Treat only a final unit variant named Other or Unknown as a candidate.
    let fallback = item.variants.last()?;
    if !fallback.is_unit || !matches!(fallback.ident.name.as_str(), "Other" | "Unknown") {
        return None;
    }

    // Skip variants intentionally excluded from deserialization.
    (!has_serde_attr(fallback.attrs, "skip")
        && !has_serde_attr(fallback.attrs, "skip_deserializing"))
    .then_some(fallback)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
