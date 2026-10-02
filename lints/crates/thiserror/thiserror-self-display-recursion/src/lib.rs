#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for thiserror display messages that format `self`.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated, then scans each `#[error("...")]`
//! format string with thiserror's placeholder rules. A `Display` placeholder
//! that names `self`, directly or through a positional `self` argument, makes
//! the generated `Display` implementation call itself.

extern crate rustc_hir;

#[cfg(test)]
use thiserror as _;

use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass};
use thiserror_support::{
    emit, error_attrs,
    format::{Argument, FormatAttr, is_display_spec},
    thiserror_shapes,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_SELF_DISPLAY_RECURSION,
    Warn,
    "`thiserror` display message formats `self` recursively",
    ThiserrorSelfDisplayRecursion
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorSelfDisplayRecursion {
    /// Check the struct, enum, and variant messages of a `thiserror::Error` type.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve the derived shapes before reading their source attributes.
        let shapes = thiserror_shapes(cx, item);
        if shapes.is_empty() {
            return;
        }
        // An enum-level message applies to every variant without its own message.
        let enum_owner = matches!(item.kind, ItemKind::Enum(..)).then(|| item.hir_id());
        shapes
            .iter()
            .map(|shape| shape.hir_id)
            .chain(enum_owner)
            .flat_map(|owner| error_attrs(cx, owner))
            .filter_map(|(span, attr)| {
                attr.format()
                    .filter(|format| has_self_display(format))
                    .map(|_| span)
            })
            .for_each(|span| {
                emit(
                    cx,
                    THISERROR_SELF_DISPLAY_RECURSION,
                    span,
                    "`thiserror` display message formats `self` recursively",
                    "format a concrete field, or remove `self` from the message",
                    None,
                );
            });
    }
}

/// Return whether a format string displays `self`.
fn has_self_display(format: &FormatAttr) -> bool {
    let mut positional = format.positional_args();
    format.placeholders().iter().any(|placeholder| {
        let is_self_name = match &placeholder.argument {
            Argument::Named(name) => name == "self",
            // Implicit placeholders consume positional arguments in order.
            Argument::Implicit => positional
                .next()
                .is_some_and(|arg| arg.ident.as_deref() == Some("self")),
            Argument::Index(_) => false,
        };
        is_self_name && is_display_spec(&placeholder.spec)
    })
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
