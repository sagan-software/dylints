#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for thiserror tuple messages that mix `{N}` and positional arguments.
//!
//! thiserror 1 accepts a numeric placeholder such as `{0}` together with an
//! unnamed format argument on a tuple struct or variant; thiserror 2 rejects
//! the combination as ambiguous. The lint proves that a type derives
//! `thiserror::Error` through the implementation the derive generated and
//! applies thiserror 2's rule, so it runs on thiserror 1 code before a
//! migration.

extern crate rustc_hir;

#[cfg(test)]
use thiserror_v1 as _;

use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};
use thiserror_support::{
    emit, error_attrs,
    format::{Argument, FormatAttr},
    thiserror_shapes,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_TUPLE_FORMAT_POSITIONAL_AMBIGUITY,
    Warn,
    "`thiserror` tuple format mixes tuple fields and positional arguments",
    ThiserrorTupleFormatPositionalAmbiguity
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorTupleFormatPositionalAmbiguity {
    /// Check the message of every tuple struct and tuple variant.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        for shape in thiserror_shapes(cx, item) {
            // thiserror 2 allows extra positional arguments only when no field is unnamed.
            if !shape.is_tuple || shape.fields.is_empty() {
                continue;
            }
            // Inspect every error attribute after the tuple shape is known.
            error_attrs(cx, shape.hir_id)
                .into_iter()
                .filter(|(_, attr)| attr.format().is_some_and(is_ambiguous))
                .for_each(|(span, _)| {
                    emit(
                        cx,
                        THISERROR_TUPLE_FORMAT_POSITIONAL_AMBIGUITY,
                        span,
                        "`thiserror` tuple format mixes numeric fields and positional arguments",
                        "give the extra format argument a name, such as `value = value()`",
                        None,
                    );
                });
        }
    }
}

/// Apply thiserror 2's rule: a numeric placeholder with any unnamed argument.
fn is_ambiguous(format: &FormatAttr) -> bool {
    format.positional_args().next().is_some()
        && format
            .placeholders()
            .iter()
            .any(|placeholder| matches!(placeholder.argument, Argument::Index(_)))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
