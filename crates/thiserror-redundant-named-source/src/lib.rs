#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for redundant `#[source]` on a thiserror field named `source`.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated. thiserror already treats a field named
//! `source` as the source when no other field has `#[from]` or `#[source]`, so
//! the lint deletes the attribute in exactly that case.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use thiserror_support::{Fix, emit, find_attr, is_field_named, thiserror_shapes};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_REDUNDANT_NAMED_SOURCE,
    Warn,
    "`thiserror` named source field has redundant `#[source]`",
    ThiserrorRedundantNamedSource
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorRedundantNamedSource {
    /// Check every struct and variant of a type that derives `thiserror::Error`.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve each shape that thiserror proved derives the error trait.
        for shape in thiserror_shapes(cx, item) {
            // thiserror infers a `source` field only when no field is marked explicitly.
            let marked = shape
                .fields
                .iter()
                .filter(|field| {
                    find_attr(cx, field.hir_id, "source").is_some()
                        || find_attr(cx, field.hir_id, "from").is_some()
                })
                .collect::<Vec<_>>();
            let [field] = marked.as_slice() else {
                continue;
            };
            let Some(source) = find_attr(cx, field.hir_id, "source") else {
                continue;
            };
            if !is_field_named(cx, field, "source") || find_attr(cx, field.hir_id, "from").is_some()
            {
                continue;
            }
            emit(
                cx,
                THISERROR_REDUNDANT_NAMED_SOURCE,
                source,
                "a field named `source` is already the error source",
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
