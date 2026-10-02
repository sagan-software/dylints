#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for raw identifiers in thiserror format placeholders.
//!
//! thiserror 1 accepts `{r#type}` in an `#[error("...")]` message, and
//! thiserror 2 rejects it. The lint proves that a type derives
//! `thiserror::Error` through the implementation the derive generated, so it
//! runs on thiserror 1 code before a migration. Removing `r#` keeps a
//! non-keyword name such as `{r#kind}` valid in both versions; a keyword such
//! as `{type}` compiles only with thiserror 2, so that rewrite is not applied
//! automatically.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::{Item, ItemKind};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_span::Symbol;
use thiserror_support::{
    Fix, emit, error_attrs,
    format::{Argument, ErrorAttr, FormatAttr},
    thiserror_shapes,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_RAW_FIELD_FORMAT,
    Warn,
    "`thiserror` format string uses a raw field identifier",
    ThiserrorRawFieldFormat
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorRawFieldFormat {
    /// Check the struct, enum, and variant messages of a `thiserror::Error` type.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve only shapes that thiserror proved derive the error trait.
        let shapes = thiserror_shapes(cx, item);
        // Include an enum owner so variant-level attributes are visited once.
        let enum_owner =
            (!shapes.is_empty() && matches!(item.kind, ItemKind::Enum(..))).then(|| item.hir_id());
        // Classify raw placeholders before choosing fix applicability.
        for (span, attr) in shapes
            .iter()
            .map(|shape| shape.hir_id)
            .chain(enum_owner)
            .flat_map(|owner| error_attrs(cx, owner))
        {
            let Some(format) = attr.format() else {
                continue;
            };
            let raw_names = format
                .placeholders()
                .into_iter()
                .filter_map(|placeholder| match placeholder.argument {
                    Argument::Named(name) => name.strip_prefix("r#").map(str::to_owned),
                    Argument::Implicit | Argument::Index(_) => None,
                })
                .collect::<Vec<_>>();
            if raw_names.is_empty() {
                continue;
            }
            // A keyword placeholder such as `{type}` does not compile with thiserror 1.
            let has_keyword = raw_names
                .iter()
                .any(|name| Symbol::intern(name).is_reserved(|| cx.sess().edition()));
            let applicability = if has_keyword {
                Applicability::MaybeIncorrect
            } else {
                Applicability::MachineApplicable
            };
            let fix = unraw(&attr, format).map(|replacement| Fix {
                span,
                replacement,
                applicability,
            });
            emit(
                cx,
                THISERROR_RAW_FIELD_FORMAT,
                span,
                "`thiserror` 2 expects unraw field names in format strings",
                "remove `r#` inside the format placeholder",
                fix,
            );
        }
    }
}

/// Rewrite the attribute text without the `r#` of each raw placeholder.
fn unraw(attr: &ErrorAttr, format: &FormatAttr) -> Option<String> {
    // Preserve all attribute text while edits are applied from right to left.
    let mut text = attr.text.clone();
    // Remove from the end so earlier offsets stay valid.
    for placeholder in format.placeholders().iter().rev() {
        if matches!(&placeholder.argument, Argument::Named(name) if name.starts_with("r#")) {
            // The source range identifies only the raw marker inside the placeholder.
            let range = format.source_range(&placeholder.range)?;
            text.replace_range(range.start + 1..range.start + 3, "");
        }
    }
    Some(text)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
