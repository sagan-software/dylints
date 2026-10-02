#![feature(rustc_private)]

//! A lint to check for Serde enum variants that fail when serialized.
//!
//! This Dylint library finds `skip` and `skip_serializing` on variants of
//! enums that derive `Serialize`, because the derived code returns an error
//! when it serializes such a variant. Each attribute is reported on the
//! variant that carries it.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{AdtKind, Help, emit_lint, serde_attr, serde_item};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_SKIP_SERIALIZING_VARIANT_ERROR,
    Warn,
    "`serde(skip)` or `serde(skip_serializing)` variant errors if serialized",
    SerdeSkipSerializingVariantError
}

/// Variant attribute keys that make `serde_derive` return an error when
/// serializing the variant, each paired with its diagnostic message.
const SKIP_KEYS: [(&str, &str); 2] = [
    (
        "skip",
        "`skip` makes this variant fail during serialization",
    ),
    (
        "skip_serializing",
        "`skip_serializing` makes this variant fail during serialization",
    ),
];

impl<'tcx> LateLintPass<'tcx> for SerdeSkipSerializingVariantError {
    /// Check the variants of one serializable enum.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only serializable enums can hit the runtime error.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if item.kind != AdtKind::Enum || !item.derives.has_serialize {
            return;
        }

        for variant in &item.variants {
            // `serde_derive` treats both keys as serialization skips that error at runtime.
            for (key, message) in SKIP_KEYS {
                let Some(skip_attr) = serde_attr(variant.attrs, key) else {
                    continue;
                };
                emit_lint(
                    cx,
                    SERDE_SKIP_SERIALIZING_VARIANT_ERROR,
                    variant.hir_id,
                    skip_attr.span(),
                    message,
                    Help::text(
                        "remove the attribute so the variant serializes, or keep this variant away from serialization",
                    ),
                );
            }
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
