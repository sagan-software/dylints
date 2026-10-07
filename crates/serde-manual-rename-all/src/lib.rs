#![feature(rustc_private)]

//! A lint to check for repeated Serde field renames replaceable by `rename_all`.
//!
//! This Dylint library compares every field's explicit Serde name with the
//! conventions `rename_all` supports, and suggests one container rule when a
//! single convention reproduces every name in each derived direction.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

use serde_support::{
    AdtKind, SerdeField, SerdeItem, attr_deletion_span, has_serde_attr, is_deletable_attr,
    serde_attr, serde_directional_value, serde_item,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_MANUAL_RENAME_ALL,
    Warn,
    "repeated serde field renames can use a container rename_all attribute",
    SerdeManualRenameAll
}

/// The field rename conventions that change a snake-case Rust field name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RenameRule {
    /// Convert snake-case fields to `PascalCase`.
    Pascal,
    /// Convert snake-case fields to `camelCase`.
    Camel,
    /// Convert ASCII letters to uppercase while retaining underscores.
    ScreamingSnake,
    /// Replace underscores with hyphens.
    Kebab,
    /// Uppercase letters and replace underscores with hyphens.
    ScreamingKebab,
}

impl RenameRule {
    /// Conventions in the order they are tried. `UPPERCASE` is omitted because it
    /// renames a snake-case field exactly like `SCREAMING_SNAKE_CASE`.
    const ALL: [Self; 5] = [
        Self::Pascal,
        Self::Camel,
        Self::ScreamingSnake,
        Self::Kebab,
        Self::ScreamingKebab,
    ];

    /// Return Serde's spelling for this rule.
    const fn name(self) -> &'static str {
        match self {
            Self::Pascal => "PascalCase",
            Self::Camel => "camelCase",
            Self::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            Self::Kebab => "kebab-case",
            Self::ScreamingKebab => "SCREAMING-KEBAB-CASE",
        }
    }

    /// Apply Serde 1.0.228's field conversion for this rule to an ASCII field name.
    fn apply(self, field: &str) -> String {
        match self {
            Self::Pascal => pascal_case(field),
            Self::Camel => {
                // Serde lowercases the first byte of the Pascal-case spelling.
                let mut camel = pascal_case(field);
                if let Some(first) = camel.get_mut(..1) {
                    first.make_ascii_lowercase();
                }
                camel
            }
            Self::ScreamingSnake => field.to_ascii_uppercase(),
            Self::Kebab => field.replace('_', "-"),
            Self::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

/// Convert one snake-case Rust field identifier to `PascalCase`.
fn pascal_case(field: &str) -> String {
    // Capitalize the first character and every character following an underscore.
    let mut pascal = String::new();
    let mut should_capitalize = true;

    for ch in field.chars() {
        // Drop separators while preserving all nonseparator characters.
        if ch == '_' {
            should_capitalize = true;
        } else if should_capitalize {
            pascal.push(ch.to_ascii_uppercase());
            should_capitalize = false;
        } else {
            pascal.push(ch);
        }
    }

    pascal
}

impl<'tcx> LateLintPass<'tcx> for SerdeManualRenameAll {
    /// Check one struct for a common explicit rename convention.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Some(item) = serde_item(cx, item) else {
            return;
        };
        check_struct(cx, &item);
    }
}

/// Check one struct for a common explicit rename convention.
fn check_struct(cx: &LateContext<'_>, item: &SerdeItem<'_>) {
    // Find the container rule that reproduces every explicit field name.
    let Some(container_attr) = container_rule(item) else {
        return;
    };
    // Anchor the consolidated suggestion at the first field-level rename.
    let Some(first_attr) = item
        .fields
        .iter()
        .find_map(|field| serde_attr(field.attrs, "rename"))
    else {
        return;
    };

    // Offer the rewrite only when deleting every field attribute is exact.
    let rewrite = rename_all_rewrite(cx, item, &container_attr);
    let help = format!("add `{container_attr}` to the struct and remove its field-level renames");
    cx.emit_span_lint(
        SERDE_MANUAL_RENAME_ALL,
        first_attr.span(),
        DiagDecorator(move |diagnostic| {
            let _configured =
                diagnostic.primary_message("all fields use one Serde rename convention");
            if let Some(parts) = rewrite {
                let _configured =
                    diagnostic.multipart_suggestion(help, parts, Applicability::MachineApplicable);
            } else {
                let _configured = diagnostic.help(help);
            }
        }),
    );
}

/// Return the container attribute that replaces every field rename, if one exists.
fn container_rule(item: &SerdeItem<'_>) -> Option<String> {
    // Keep only multi-field structs with named fields and no container rule.
    if item.kind != AdtKind::Struct || !has_renamable_fields(item) {
        return None;
    }
    if has_serde_attr(item.attrs, "rename_all") {
        return None;
    }

    // Each derived direction must be fully representable before field attributes can go away.
    let serialize_rule = common_rule(&item.fields, Direction::Serialize);
    let deserialize_rule = common_rule(&item.fields, Direction::Deserialize);
    let is_serialize_covered = !item.derives.has_serialize || serialize_rule.is_some();
    let is_deserialize_covered = !item.derives.has_deserialize || deserialize_rule.is_some();
    if !(is_serialize_covered && is_deserialize_covered) {
        return None;
    }
    // Keep only the rules of derived directions.
    container_attr(
        serialize_rule.filter(|_| item.derives.has_serialize),
        deserialize_rule.filter(|_| item.derives.has_deserialize),
    )
}

/// Return whether a struct has enough named fields for a container rule.
fn has_renamable_fields(item: &SerdeItem<'_>) -> bool {
    item.fields.len() >= 2 && item.fields.iter().all(|field| field.name.is_some())
}

/// One Serde operation direction.
#[derive(Clone, Copy, Debug)]
enum Direction {
    /// Serialization names.
    Serialize,
    /// Deserialization names.
    Deserialize,
}

/// Find one rule that produces every field's explicit name in a direction.
fn common_rule(fields: &[SerdeField<'_>], direction: Direction) -> Option<RenameRule> {
    RenameRule::ALL.into_iter().find(|rule| {
        fields.iter().all(|field| {
            let Some(name) = field.name else {
                return false;
            };
            let name = name.as_str();
            let Some(rename) = serde_directional_value(field.attrs, "rename") else {
                return false;
            };
            let explicit = match direction {
                Direction::Serialize => rename.serialize,
                Direction::Deserialize => rename.deserialize,
            };

            name.is_ascii() && explicit.is_some_and(|explicit| rule.apply(name) == explicit)
        })
    })
}

/// Return the shortest container attribute that preserves every derived direction.
///
/// Each argument is `None` exactly when its direction is not derived.
fn container_attr(
    serialize: Option<RenameRule>,
    deserialize: Option<RenameRule>,
) -> Option<String> {
    Some(match (serialize, deserialize) {
        // Equal bidirectional rules use Serde's shared shorthand.
        (Some(serialize), Some(deserialize)) if serialize == deserialize => {
            format!("#[serde(rename_all = \"{}\")]", serialize.name())
        }
        // Distinct bidirectional rules remain explicit inside the directional form.
        (Some(serialize), Some(deserialize)) => format!(
            "#[serde(rename_all(serialize = \"{}\", deserialize = \"{}\"))]",
            serialize.name(),
            deserialize.name()
        ),
        // Single-direction derives retain only their derived rename rule.
        (Some(serialize), None) => {
            format!("#[serde(rename_all(serialize = \"{}\"))]", serialize.name())
        }
        (None, Some(deserialize)) => format!(
            "#[serde(rename_all(deserialize = \"{}\"))]",
            deserialize.name()
        ),
        (None, None) => return None,
    })
}

/// Build the exact rewrite: insert the container rule and delete each field rename.
///
/// The rewrite is exact only when every field's rename attribute holds just
/// `rename` and comes from plain source.
fn rename_all_rewrite(
    cx: &LateContext<'_>,
    item: &SerdeItem<'_>,
    container_attr: &str,
) -> Option<Vec<(Span, String)>> {
    // Macro output has no source text to rewrite.
    if item.span.from_expansion() {
        return None;
    }
    // Insert the container attribute on its own line at the item's indentation.
    let indent = cx.sess().source_map().indentation_before(item.span)?;
    let mut parts = vec![(
        item.span.shrink_to_lo(),
        format!("{container_attr}\n{indent}"),
    )];
    // Delete every field rename, and give up if any deletion would be inexact.
    for field in &item.fields {
        let attr = serde_attr(field.attrs, "rename")?;
        if !is_deletable_attr(cx, attr, "rename") {
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

#[cfg(test)]
mod tests {
    use super::RenameRule;

    /// Apply each rule to a multi-word field.
    #[test]
    fn rules_match_serde() {
        let applied: Vec<_> = RenameRule::ALL
            .into_iter()
            .map(|rule| rule.apply("user_id"))
            .collect();
        assert_eq!(
            applied,
            ["UserId", "userId", "USER_ID", "user-id", "USER-ID"]
        );
        assert_eq!(RenameRule::Camel.apply(""), "");
    }
}
