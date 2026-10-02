#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for thiserror wrappers that display only their source.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated. It then reports a struct or variant
//! whose only field is its source and whose message only displays that field.
//! The suggested `#[error(transparent)]` changes what `source()` returns, so the
//! suggestion is never applied automatically.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};
use thiserror_support::{
    ErrorShape, Fix, emit, error_attrs, find_attr,
    format::{Argument, FormatAttr},
    is_field_named, thiserror_shapes,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_TRANSPARENT_SINGLE_SOURCE_DISPLAY,
    Warn,
    "`thiserror` wrapper manually forwards source display",
    ThiserrorTransparentSingleSourceDisplay
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorTransparentSingleSourceDisplay {
    /// Check every struct and variant of a type that derives `thiserror::Error`.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve each shape that thiserror proved derives the error trait.
        for shape in thiserror_shapes(cx, item) {
            let [field] = shape.fields else {
                continue;
            };
            // Treat explicit source markers and the conventional field name alike.
            let has_source_attr = find_attr(cx, field.hir_id, "source").is_some();
            let is_source = has_source_attr
                || find_attr(cx, field.hir_id, "from").is_some()
                || is_field_named(cx, field, "source");
            if !is_source {
                continue;
            }
            // Inspect each error message for the sole source placeholder.
            for (span, attr) in error_attrs(cx, shape.hir_id) {
                if !attr
                    .format()
                    .is_some_and(|format| displays_only_field(format, &shape, field.ident.as_str()))
                {
                    continue;
                }
                // thiserror rejects `#[source]` inside a transparent item.
                let fix = (!has_source_attr).then(|| Fix {
                    span,
                    replacement: "#[error(transparent)]".to_owned(),
                    applicability: Applicability::MaybeIncorrect,
                });
                emit(
                    cx,
                    THISERROR_TRANSPARENT_SINGLE_SOURCE_DISPLAY,
                    span,
                    "`thiserror` can forward this source transparently",
                    "replace the manual display forwarding with `#[error(transparent)]`",
                    fix,
                );
            }
        }
    }
}

/// Return whether a message is exactly one plain placeholder for the only field.
fn displays_only_field(format: &FormatAttr, shape: &ErrorShape<'_>, name: &str) -> bool {
    // Require one placeholder before comparing its argument and source range.
    let placeholders = format.placeholders();
    let [placeholder] = placeholders.as_slice() else {
        return false;
    };
    let names_field = match &placeholder.argument {
        Argument::Index(0) => shape.is_tuple,
        Argument::Named(placeholder_name) => !shape.is_tuple && placeholder_name == name,
        Argument::Index(_) | Argument::Implicit => false,
    };
    names_field
        && format.args.is_empty()
        && placeholder.spec.is_empty()
        && placeholder.range == (0..format.value.len())
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
