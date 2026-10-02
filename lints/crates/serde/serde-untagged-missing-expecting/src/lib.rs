#![feature(rustc_private)]

//! A lint to check for untagged Serde enums without an `expecting` message.
//!
//! This Dylint library finds enums that derive `Deserialize` with
//! `#[serde(untagged)]` and no `#[serde(expecting = "...")]`, whose errors
//! otherwise give no hint about the accepted input. The diagnostic points
//! at the `untagged` attribute that makes the message necessary.

extern crate rustc_hir;

#[cfg(test)]
use serde as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};

use serde_support::{AdtKind, Help, emit_lint, has_serde_attr, serde_attr, serde_item};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_UNTAGGED_MISSING_EXPECTING,
    Warn,
    "`serde(untagged)` enum should provide an expecting message",
    SerdeUntaggedMissingExpecting
}

impl<'tcx> LateLintPass<'tcx> for SerdeUntaggedMissingExpecting {
    /// Check one untagged deserializable enum for a custom expecting message.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Only a deserializable enum produces the untagged fallback error.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        if item.kind != AdtKind::Enum || !item.derives.has_deserialize {
            return;
        }
        let Some(untagged_attr) = serde_attr(item.attrs, "untagged") else {
            return;
        };
        if has_serde_attr(item.attrs, "expecting") {
            return;
        }

        // Point at `untagged`, which is the attribute that makes the message necessary.

        emit_lint(
            cx,
            SERDE_UNTAGGED_MISSING_EXPECTING,
            cx.tcx.local_def_id_to_hir_id(item.def_id),
            untagged_attr.span(),
            "`serde(untagged)` does not produce informative fallback errors by default",
            Help::text(
                "add `expecting = \"...\"` with a domain-specific description of accepted input",
            ),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
