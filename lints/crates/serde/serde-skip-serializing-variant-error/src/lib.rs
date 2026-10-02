#![feature(rustc_private)]

//! A lint to check for serde variants that error when serialized.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_lint::{EarlyContext, EarlyLintPass};

use serde_support::{ItemKind, ast_serde_attr, emit_span_lint_with_help, serde_ast_crate};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_SKIP_SERIALIZING_VARIANT_ERROR,
    Warn,
    "`serde(skip)` or `serde(skip_serializing)` variant errors if serialized",
    SerdeSkipSerializingVariantError
}

impl EarlyLintPass for SerdeSkipSerializingVariantError {
    /// Check crate for this lint.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
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

/// Check serializable enums for variants skipped during serialization.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde enums before inspecting their variant attributes.
    let krate = serde_ast_crate(cx, krate);

    // Report each serialization-only skip on an otherwise serializable enum.
    for item in &krate.items {
        if item.kind != ItemKind::Enum || !item.derives.has_serialize {
            continue;
        }

        for variant in &item.variants {
            // `serde_derive` treats both keys as serialization skips that error at runtime.
            for (key, message) in SKIP_KEYS {
                let Some(skip_attr) = ast_serde_attr(cx, variant.attrs, key) else {
                    continue;
                };
                emit_span_lint_with_help(
                    cx,
                    SERDE_SKIP_SERIALIZING_VARIANT_ERROR,
                    skip_attr.span,
                    message,
                    "remove the attribute so the variant serializes, or keep this variant away from serialization",
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
