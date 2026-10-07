#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Inspect attribute order before expansion because documentation layout is
//! source syntax. The pass recognizes documentation comments and built-in doc
//! attributes. It reports source-authored attributes that interrupt one
//! documentation block. Automatic edits move one built-in attribute across
//! documentation only. Other attribute orders require manual review because
//! procedural macros can observe them.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{AttrKind, Attribute};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::sym;

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub INTERLEAVED_DOC_ATTRIBUTES,
    Warn,
    "non-documentation attribute interrupts a documentation block",
    InterleavedDocAttributes
}

impl EarlyLintPass for InterleavedDocAttributes {
    /// Report source attributes between the first and last documentation attributes.
    fn check_attributes(&mut self, cx: &EarlyContext<'_>, attributes: &[Attribute]) {
        // Locate the complete documentation block before inspecting interruptions.
        let first = attributes.iter().position(is_documentation);
        let last = attributes
            .iter()
            .enumerate()
            .rfind(|(_, attribute)| is_documentation(attribute));
        let (Some(first), Some((last, documentation_end))) = (first, last) else {
            return;
        };
        // Expansion can reorder or synthesize attributes without editable source.
        for attribute in attributes.iter().take(last).skip(first) {
            if !is_documentation(attribute) && !attribute.span.from_expansion() {
                // Unknown attributes may inspect order, so only known built-ins are editable.
                let can_reorder = attributes
                    .iter()
                    .all(|attribute| is_documentation(attribute) || is_reorderable(attribute));
                let interrupted_count = attributes
                    .iter()
                    .take(last)
                    .skip(first)
                    .filter(|attribute| !is_documentation(attribute))
                    .count();
                let replacement = (can_reorder && interrupted_count == 1)
                    .then(|| safe_replacement(cx, attribute, documentation_end))
                    .flatten();
                // Multiple interruptions retain a diagnostic without an automatic edit.
                cx.emit_span_lint(INTERLEAVED_DOC_ATTRIBUTES, attribute.span, DiagDecorator(|diagnostic| {
                    let _message = diagnostic.primary_message("non-documentation attribute interrupts a documentation block");
                    if let Some((span, replacement)) = replacement {
                        let _suggestion = diagnostic.span_suggestion(span, "keep the documentation block contiguous", replacement, Applicability::MachineApplicable);
                    } else {
                        let _help = diagnostic.help("move this attribute below the complete documentation block; review procedural macro attribute order before moving it");
                    }
                }));
            }
        }
    }
}

/// Recognize built-in metadata that cannot execute a procedural macro.
fn is_reorderable(attribute: &Attribute) -> bool {
    [sym::must_use, sym::inline, sym::cold]
        .iter()
        .any(|name| attribute.has_name(*name))
}

/// Rewrite one built-in attribute only when intervening source remains unchanged.
fn safe_replacement(
    cx: &EarlyContext<'_>,
    attribute: &Attribute,
    last: &Attribute,
) -> Option<(rustc_span::Span, String)> {
    // Procedural macros can observe attribute order, so unknown attributes require review.
    if !is_reorderable(attribute)
        || last.span.from_expansion()
        || attribute.span.ctxt() != last.span.ctxt()
    {
        return None;
    }
    let span = attribute.span.with_hi(last.span.hi());
    let source = cx.sess().source_map();
    let original = source.span_to_snippet(span).ok()?;
    let attribute_source = source.span_to_snippet(attribute.span).ok()?;
    let documentation = original
        .strip_prefix(&attribute_source)?
        .trim_start_matches([' ', '\t', '\n', '\r']);
    // Preserve all comments and documentation spelling while moving the exact attribute.
    Some((span, format!("{documentation}\n{attribute_source}")))
}

/// Include both documentation comments and explicit documentation attributes.
fn is_documentation(attribute: &Attribute) -> bool {
    matches!(attribute.kind, AttrKind::DocComment(..)) || attribute.has_name(sym::doc)
}

/// Exercise triggering and contiguous documentation through the compiler driver.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
