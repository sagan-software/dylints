#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for thiserror named fields formatted through positional arguments.
//!
//! The lint proves that a type derives `thiserror::Error` through the
//! implementation the derive generated. It pairs each implicit `{}` placeholder
//! with its positional argument and reports an argument that is just the name
//! of a field. When the message has exactly that one placeholder and argument,
//! the fix moves the field name into the placeholder and deletes the argument.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use thiserror as _;

use rustc_errors::Applicability;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;
use thiserror_support::{
    ErrorShape, Fix, emit, error_attrs,
    format::{Argument, ErrorAttr, FormatArg, FormatAttr, Placeholder},
    is_field_named, thiserror_shapes,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub THISERROR_NAMED_FIELD_POSITIONAL_FORMAT,
    Warn,
    "`thiserror` named field is formatted positionally",
    ThiserrorNamedFieldPositionalFormat
}

impl<'tcx> LateLintPass<'tcx> for ThiserrorNamedFieldPositionalFormat {
    /// Check every message of a struct or variant with named fields.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve only shapes that thiserror proved derive the error trait.
        for shape in thiserror_shapes(cx, item) {
            if shape.is_tuple {
                continue;
            }
            // Inspect each error message for an implicit field argument.
            for (span, attr) in error_attrs(cx, shape.hir_id) {
                let Some(format) = attr.format() else {
                    continue;
                };
                if !is_field_formatted_positionally(cx, format, &shape) {
                    continue;
                }
                emit(
                    cx,
                    THISERROR_NAMED_FIELD_POSITIONAL_FORMAT,
                    span,
                    "this named field is formatted positionally",
                    "capture the field directly as `{field}`",
                    capture_fix(span, &attr, format),
                );
            }
        }
    }
}

/// Return whether an implicit placeholder's argument is the bare name of a field.
fn is_field_formatted_positionally(
    cx: &LateContext<'_>,
    format: &FormatAttr,
    shape: &ErrorShape<'_>,
) -> bool {
    let placeholders = format.placeholders();
    let implicit = placeholders
        .iter()
        .filter(|placeholder| placeholder.argument == Argument::Implicit);
    // Implicit placeholders consume positional arguments in order.
    implicit.zip(format.positional_args()).any(|(_, arg)| {
        arg.ident.as_deref().is_some_and(|name| {
            shape
                .fields
                .iter()
                .any(|field| is_field_named(cx, field, name))
        })
    })
}

/// Build the exact rewrite for a message with one placeholder and one positional argument.
fn capture_fix(span: Span, attr: &ErrorAttr, format: &FormatAttr) -> Option<Fix> {
    // Require one implicit placeholder before rewriting the source text.
    let placeholder = single_implicit_placeholder(format)?;
    // Require one positional argument so the rewrite cannot shift a later value.
    let arg = single_positional_argument(format)?;
    // The argument must be a named field and map to a source range.
    let (name, placeholder_range) = field_name_and_range(format, &placeholder, arg)?;
    let (before, kept, after) = source_parts(attr, placeholder_range, arg)?;
    Some(Fix {
        span,
        replacement: format!(
            "{before}{{{name}{spec}}}{kept}{after}",
            spec = placeholder.spec
        ),
        applicability: Applicability::MachineApplicable,
    })
}

/// Return the field name and source range used by the rewrite.
fn field_name_and_range<'a>(
    format: &'a FormatAttr,
    placeholder: &Placeholder,
    arg: &'a FormatArg,
) -> Option<(&'a str, std::ops::Range<usize>)> {
    let name = arg.ident.as_deref()?;
    let source_range = format.source_range(&placeholder.range)?;
    Some((name, source_range))
}

/// Return the only implicit placeholder when all other placeholders are named.
fn single_implicit_placeholder(format: &FormatAttr) -> Option<Placeholder> {
    // Another positional placeholder could shift which value each one formats.
    let placeholders = format
        .placeholders()
        .into_iter()
        .filter(|placeholder| !matches!(placeholder.argument, Argument::Named(_)))
        .collect::<Vec<_>>();
    let [placeholder] = placeholders.as_slice() else {
        return None;
    };
    (placeholder.argument == Argument::Implicit).then(|| placeholder.clone())
}

/// Return the only positional format argument.
fn single_positional_argument(format: &FormatAttr) -> Option<&FormatArg> {
    let mut positional = format.positional_args();
    let (Some(arg), None) = (positional.next(), positional.next()) else {
        return None;
    };
    Some(arg)
}

/// Keep the source slices around the deleted positional argument.
fn source_parts<'a>(
    attr: &'a ErrorAttr,
    placeholder: std::ops::Range<usize>,
    arg: &FormatArg,
) -> Option<(&'a str, &'a str, &'a str)> {
    let before = attr.text.get(..placeholder.start)?;
    let kept = attr.text.get(placeholder.end..arg.previous_end)?;
    let after = attr.text.get(arg.range.end..)?;
    Some((before, kept, after))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
