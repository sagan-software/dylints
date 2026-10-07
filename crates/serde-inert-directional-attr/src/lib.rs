#![feature(rustc_private)]

//! A lint to check for serde attributes that cannot affect the derived direction.
//!
//! This Dylint library resolves which Serde traits a type derives and reports
//! container, field, and variant entries that affect only the direction the
//! type does not derive. When source spans prove active values and comments will
//! be preserved, it suggests deleting only the inert entries.

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_lexer;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_ast::{LitKind, MetaItemInner, MetaItemKind};
use rustc_hir::{Attribute, HirId, Item};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_span::Span;

use serde_support::{
    Help, SerdeItem, all_fields, attr_has_entry, emit_lint, namespace_attrs, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_INERT_DIRECTIONAL_ATTR,
    Warn,
    "`serde` attribute cannot affect the only derived direction",
    SerdeInertDirectionalAttr
}

impl<'tcx> LateLintPass<'tcx> for SerdeInertDirectionalAttr {
    /// Check one item that derives exactly one Serde direction.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        // Select the keys that affect only the direction absent from this item.
        let (keys, inactive_direction, help) = if item.derives.only_serialize() {
            (
                DESERIALIZE_ONLY_KEYS,
                SerdeDirection::Deserialize,
                "remove the deserialization-only attribute, or add `Deserialize` if it was intended",
            )
        } else if item.derives.only_deserialize() {
            (
                SERIALIZE_ONLY_KEYS,
                SerdeDirection::Serialize,
                "remove the serialization-only attribute, or add `Serialize` if it was intended",
            )
        } else {
            // Preserve attributes when both directions are derived or neither is derived.
            return;
        };

        // Resolve both flat keys and nested direction entries on this attribute.
        for (hir_id, attr) in directional_attrs(cx, &item) {
            let direct_key = keys.iter().find(|key| attr_has_entry(attr, key));
            let directional_key = DIRECTIONAL_KEYS
                .iter()
                .find(|key| has_inert_directional_value(attr, key, inactive_direction));
            let Some(key) = direct_key.or(directional_key) else {
                continue;
            };

            // Keep existing whole-attribute fixes and prove separate nested-entry fixes.
            let help = if direct_key.is_some() {
                Help::attr_deletion(cx, attr, key, help)
            } else {
                directional_attr_help(cx, attr, key, inactive_direction)
            };
            emit_lint(
                cx,
                SERDE_INERT_DIRECTIONAL_ATTR,
                hir_id,
                attr.span(),
                "`serde` attribute is inert for the only derived direction",
                help,
            );
        }
    }
}

/// One Serde derive direction.
#[derive(Clone, Copy)]
enum SerdeDirection {
    /// Serde serialization.
    Serialize,
    /// Serde deserialization.
    Deserialize,
}

impl SerdeDirection {
    /// Return the nested attribute name for this direction.
    const fn key(self) -> &'static str {
        match self {
            Self::Serialize => "serialize",
            Self::Deserialize => "deserialize",
        }
    }

    /// Describe removing an inert directional subentry.
    const fn help(self) -> &'static str {
        match self {
            Self::Serialize => {
                "remove the `serialize` entry, or add `Serialize` if it was intended"
            }
            Self::Deserialize => {
                "remove the `deserialize` entry, or add `Deserialize` if it was intended"
            }
        }
    }

    /// Return the other Serde derive direction.
    const fn opposite(self) -> Self {
        match self {
            Self::Serialize => Self::Deserialize,
            Self::Deserialize => Self::Serialize,
        }
    }
}

/// Return container, field, and variant attributes with their owning node.
fn directional_attrs<'tcx>(
    cx: &LateContext<'tcx>,
    item: &SerdeItem<'tcx>,
) -> Vec<(HirId, &'tcx Attribute)> {
    // Container attributes come first, so diagnostics follow source order.
    let item_hir_id = cx.tcx.local_def_id_to_hir_id(item.def_id);
    let mut attrs: Vec<_> = namespace_attrs(item.attrs, "serde")
        .map(|attr| (item_hir_id, attr))
        .collect();
    // Field attributes carry the field node so `allow` on a field applies.
    for field in all_fields(item) {
        attrs.extend(namespace_attrs(field.attrs, "serde").map(|attr| (field.hir_id, attr)));
    }
    for variant in &item.variants {
        attrs.extend(namespace_attrs(variant.attrs, "serde").map(|attr| (variant.hir_id, attr)));
    }
    attrs
}

/// Return whether `key` contains a string value for the inactive direction.
fn has_inert_directional_value(
    attr: &Attribute,
    key: &str,
    inactive_direction: SerdeDirection,
) -> bool {
    serde_support::attr_entries(attr, key).any(|entry| {
        entry.meta_item_list().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| is_directional_value(entry, inactive_direction))
        })
    })
}

/// Match a Serde nested direction entry with a string value.
fn is_directional_value(entry: &MetaItemInner, direction: SerdeDirection) -> bool {
    let Some(meta) = entry.meta_item() else {
        return false;
    };
    entry_key(entry) == Some(direction.key())
        && matches!(
            &meta.kind,
            MetaItemKind::NameValue(value) if matches!(&value.kind, LitKind::Str(..))
        )
}

/// Return the single-segment name of a structured Serde entry.
fn entry_key(entry: &MetaItemInner) -> Option<&str> {
    let meta = entry.meta_item()?;
    let [segment] = meta.path.segments.as_slice() else {
        return None;
    };
    Some(segment.ident.name.as_str())
}

/// If source spans prove an exact deletion, attach a machine fix to
/// inactive-entry help.
fn directional_attr_help(
    cx: &LateContext<'_>,
    attr: &Attribute,
    key: &str,
    inactive_direction: SerdeDirection,
) -> Help {
    let help = inactive_direction.help();
    // Keep expanded or generated attributes diagnostic-only.
    if !serde_support::is_plain_source_attr(cx, attr, "serde") {
        return Help::text(help);
    }
    // Parse the source attribute before proposing any deletion span.
    let Some(outer_entries) = attr.meta_item_list() else {
        return Help::text(help);
    };
    rewrite_directional_entry(cx, attr, key, inactive_direction, &outer_entries, help)
}

/// If every sibling selects a known direction, rewrite the inactive nested entry.
fn rewrite_directional_entry(
    cx: &LateContext<'_>,
    attr: &Attribute,
    key: &str,
    inactive_direction: SerdeDirection,
    outer_entries: &[MetaItemInner],
    help: &str,
) -> Help {
    // Reject duplicate parent keys and malformed lists before computing source spans.
    let Some((outer_index, inner_entries)) = unique_directional_list(outer_entries, key) else {
        return Help::text(help);
    };

    // Reject duplicates and retain only known active-direction siblings.
    let Some(inactive_index) = unique_directional_entry(inner_entries, inactive_direction) else {
        return Help::text(help);
    };
    if !has_only_active_directional_siblings(
        inner_entries,
        inactive_index,
        inactive_direction.opposite(),
    ) {
        return Help::text(help);
    }

    // If no active nested value remains, delete the parent entry.
    if inner_entries.len() == 1 {
        return rewrite_directional_parent(cx, attr, outer_entries, outer_index, key, help);
    }

    // If the siblings select the active direction, remove only the inactive value.
    rewrite_entry(cx, inner_entries, inactive_index, help)
}

/// Find a unique parent entry and its nested list.
fn unique_directional_list<'a>(
    entries: &'a [MetaItemInner],
    key: &str,
) -> Option<(usize, &'a [MetaItemInner])> {
    let mut matches = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry_key(entry) == Some(key));
    let (index, entry) = matches.next()?;
    matches
        .next()
        .is_none()
        .then_some((index, entry.meta_item_list()?))
}

/// Find the sole nested entry for the inactive direction.
fn unique_directional_entry(entries: &[MetaItemInner], direction: SerdeDirection) -> Option<usize> {
    let mut matches = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| is_directional_value(entry, direction));
    let (index, _) = matches.next()?;
    matches.next().is_none().then_some(index)
}

/// Check that every other nested entry names the active direction.
fn has_only_active_directional_siblings(
    entries: &[MetaItemInner],
    inactive_index: usize,
    active_direction: SerdeDirection,
) -> bool {
    entries.iter().enumerate().all(|(index, entry)| {
        index == inactive_index || is_directional_value(entry, active_direction)
    })
}

/// If the parent is the only outer entry, delete it; otherwise delete its list
/// entry.
fn rewrite_directional_parent(
    cx: &LateContext<'_>,
    attr: &Attribute,
    outer_entries: &[MetaItemInner],
    outer_index: usize,
    key: &str,
    help: &str,
) -> Help {
    if outer_entries.len() == 1 {
        // If deleting the whole attribute would remove a comment, keep the help non-machine.
        if span_contains_comment(cx, attr.span()) {
            return Help::text(help);
        }
        return Help::attr_deletion(cx, attr, key, help);
    }
    rewrite_entry(cx, outer_entries, outer_index, help)
}

/// Build an exact deletion for one list entry and its adjacent comma.
fn rewrite_entry(
    cx: &LateContext<'_>,
    entries: &[MetaItemInner],
    index: usize,
    help: &str,
) -> Help {
    let Some(span) = list_entry_deletion_span(cx, entries, index) else {
        return Help::text(help);
    };
    Help::Rewrite {
        text: help.to_owned(),
        parts: vec![(span, String::new())],
    }
}

/// If its separator contains only a comma and whitespace, return an entry
/// deletion span.
fn list_entry_deletion_span(
    cx: &LateContext<'_>,
    entries: &[MetaItemInner],
    index: usize,
) -> Option<Span> {
    // If the separator contains comments or other tokens, refuse the edit
    // without dropping it.
    // If there is a previous sibling, remove its leading comma. Otherwise, remove
    // its trailing comma.
    let (separator, deletion) = index.checked_sub(1).map_or_else(
        || entry_span_with_next(entries, index),
        |previous_index| entry_span_with_previous(entries, previous_index, index),
    )?;
    let source_map = cx.sess().source_map();
    let snippet = source_map.span_to_snippet(separator).ok()?;
    // If the deletion span contains a comment token, preserve it and withhold the machine edit.
    if span_contains_comment(cx, deletion) {
        return None;
    }
    is_comma_then_whitespace(&snippet).then_some(deletion)
}

/// Return whether a source span contains a comment token.
fn span_contains_comment(cx: &LateContext<'_>, span: Span) -> bool {
    let Ok(source) = cx.sess().source_map().span_to_snippet(span) else {
        return true;
    };
    tokenize(&source, FrontmatterAllowed::No).any(|token| {
        matches!(
            token.kind,
            TokenKind::LineComment { .. } | TokenKind::BlockComment { .. }
        )
    })
}

/// If an entry has a previous sibling, build its deletion and separator spans.
fn entry_span_with_previous(
    entries: &[MetaItemInner],
    previous_index: usize,
    index: usize,
) -> Option<(Span, Span)> {
    let previous = entries.get(previous_index)?;
    let entry = entries.get(index)?;
    // If this is not the first entry, remove its leading comma with its source span.
    let separator = previous
        .span()
        .with_lo(previous.span().hi())
        .with_hi(entry.span().lo());
    let deletion = entry.span().with_lo(previous.span().hi());
    Some((separator, deletion))
}

/// If an entry is first in its list, build its deletion and separator spans.
fn entry_span_with_next(entries: &[MetaItemInner], index: usize) -> Option<(Span, Span)> {
    let entry = entries.get(index)?;
    let next = entries.get(index + 1)?;
    // If this is the first entry, remove its trailing comma with its source span.
    let separator = entry
        .span()
        .with_lo(entry.span().hi())
        .with_hi(next.span().lo());
    let deletion = entry.span().with_hi(next.span().lo());
    Some((separator, deletion))
}

/// Match a separator that preserves every source comment and token.
fn is_comma_then_whitespace(separator: &str) -> bool {
    separator
        .strip_prefix(',')
        .is_some_and(|rest| rest.chars().all(char::is_whitespace))
}

/// Serde keys that affect only deserialization.
const DESERIALIZE_ONLY_KEYS: &[&str] = &[
    "alias",
    "default",
    "deserialize_with",
    "borrow",
    "skip_deserializing",
];
/// Serde keys that affect only serialization.
const SERIALIZE_ONLY_KEYS: &[&str] = &[
    "skip_serializing",
    "skip_serializing_if",
    "serialize_with",
    "getter",
];
/// Serde entries that accept nested serialization and deserialization values.
const DIRECTIONAL_KEYS: &[&str] = &["rename", "rename_all", "rename_all_fields", "bound"];

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
