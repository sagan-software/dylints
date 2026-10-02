#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for redundant thiserror from and source attributes.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated, then reports `#[source]` on a field
//! that already has `#[from]`, with a fix that deletes `#[source]`.
//! The check uses resolved field attributes and keeps the rewrite machine-applicable.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use thiserror_support::{Fix, emit, find_attr, thiserror_shapes};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_REDUNDANT_FROM_SOURCE,
    Warn,
    "`thiserror` field marks a source twice",
    ThiserrorRedundantFromSource
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorRedundantFromSource {
    /// Check every field of a type that derives `thiserror::Error`.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        for field in thiserror_shapes(cx, item)
            .iter()
            .flat_map(|shape| shape.fields)
        {
            // Resolve the source attribute before checking whether `from` is present.
            let Some(source) = find_attr(cx, field.hir_id, "source") else {
                continue;
            };
            if find_attr(cx, field.hir_id, "from").is_none() {
                continue;
            }
            // `#[from]` already makes the field the source, so deleting `#[source]` is exact.
            emit(
                cx,
                THISERROR_REDUNDANT_FROM_SOURCE,
                source,
                "`#[from]` already marks this field as the error source",
                "remove the redundant `#[source]` attribute",
                Some(Fix {
                    span: cx.sess().source_map().span_extend_while_whitespace(source),
                    replacement: String::new(),
                    applicability: Applicability::MachineApplicable,
                }),
            );
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
