#![feature(rustc_private)]

//! A lint to check for Serde field defaults replaceable by a container default.
//!
//! This Dylint library finds structs whose derived `Default` and `Deserialize`
//! implementations make every bare field-level `#[serde(default)]` equivalent
//! to one container-level attribute, and suggests that attribute. The rewrite
//! is offered only when it is exact, so generic structs get help instead.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_span::Span;

use serde_support::{
    AdtKind, SerdeItem, attr_deletion_span, has_serde_attr, has_serde_word, is_deletable_attr,
    serde_attr, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_ALL_FIELDS_DEFAULT,
    Warn,
    "repeated serde field defaults can use a container default attribute",
    SerdeAllFieldsDefault
}

impl<'tcx> LateLintPass<'tcx> for SerdeAllFieldsDefault {
    /// Check one struct for a bare `#[serde(default)]` on every field.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Resolve the derived traits before reading field attributes.
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        // Keep only derived-default structs whose fields can share a container default.
        let is_struct = item.kind == AdtKind::Struct;
        let has_fields = item.fields.len() >= 2;
        let is_candidate =
            is_struct && has_fields && item.derives.has_deserialize && item.derives.has_default;
        if !is_candidate || has_serde_attr(item.attrs, "default") {
            return;
        }
        let has_field_defaults = item
            .fields
            .iter()
            .all(|field| has_serde_word(field.attrs, "default"));
        if !has_field_defaults {
            return;
        }
        // Anchor the diagnostic at the first field-level attribute.
        let Some(first_attr) = item
            .fields
            .iter()
            .find_map(|field| serde_attr(field.attrs, "default"))
        else {
            return;
        };
        let rewrite = container_default_rewrite(cx, &item);
        let help = "add `#[serde(default)]` to the struct and remove its field-level defaults";

        cx.emit_span_lint(
            SERDE_ALL_FIELDS_DEFAULT,
            first_attr.span(),
            DiagDecorator(move |diag| {
                let _configured = diag.primary_message(
                    "all fields use `serde(default)` with a derived `Default` implementation",
                );
                if let Some(parts) = rewrite {
                    let _configured =
                        diag.multipart_suggestion(help, parts, Applicability::MachineApplicable);
                } else {
                    let _configured = diag.help(help);
                }
            }),
        );
    }
}

/// Build the exact rewrite: insert the container attribute and delete each field attribute.
///
/// The rewrite is exact only when every field attribute holds just `default` and comes
/// from plain source, and the struct has no type parameters. Generic structs are
/// excluded because a container default needs `Self: Default`, whose derived bounds
/// the field defaults do not need.
fn container_default_rewrite(
    cx: &LateContext<'_>,
    item: &SerdeItem<'_>,
) -> Option<Vec<(Span, String)>> {
    // Generic structs would need different bounds for a container default.
    let has_type_params = cx.tcx.generics_of(item.def_id).own_counts().types > 0;
    if item.span.from_expansion() || has_type_params {
        return None;
    }
    // Insert the container attribute on its own line at the item's indentation.
    let indent = cx.sess().source_map().indentation_before(item.span)?;
    let mut parts = vec![(
        item.span.shrink_to_lo(),
        format!("#[serde(default)]\n{indent}"),
    )];
    // Delete every field attribute, and give up if any deletion would be inexact.
    for field in &item.fields {
        let attr = serde_attr(field.attrs, "default")?;
        if !is_deletable_attr(cx, attr, "default") {
            return None;
        }
        parts.push((attr_deletion_span(cx, attr), String::new()));
    }
    Some(parts)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
