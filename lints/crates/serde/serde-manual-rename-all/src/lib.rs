#![feature(rustc_private)]

//! A lint to check for repeated Serde field renames replaceable by `rename_all`.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Serde field names, reports one common rename
//! convention written repeatedly, and recommends a container-level rule.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

use serde_support::{
    AstFieldInfo, AstItemInfo, ItemKind, ast_has_serde_attr, ast_serde_attr,
    ast_serde_directional_value, serde_ast_crate,
};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_MANUAL_RENAME_ALL,
    Warn,
    "repeated serde field renames can use a container rename_all attribute",
    SerdeManualRenameAll
}

/// The field rename conventions supported by Serde.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RenameRule {
    /// Convert ASCII letters to uppercase.
    Upper,
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
    /// All conventions that can observably rename a Rust field.
    const ALL: [Self; 6] = [
        Self::Upper,
        Self::Pascal,
        Self::Camel,
        Self::ScreamingSnake,
        Self::Kebab,
        Self::ScreamingKebab,
    ];

    /// Return Serde's spelling for this rule.
    const fn name(self) -> &'static str {
        match self {
            Self::Upper => "UPPERCASE",
            Self::Pascal => "PascalCase",
            Self::Camel => "camelCase",
            Self::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            Self::Kebab => "kebab-case",
            Self::ScreamingKebab => "SCREAMING-KEBAB-CASE",
        }
    }

    /// Apply Serde 1.0.228's field conversion for this rule.
    fn apply(self, field: &str) -> String {
        // Apply each closed rule with Serde's documented ASCII transformations.
        match self {
            Self::Upper | Self::ScreamingSnake => field.to_ascii_uppercase(),
            Self::Pascal => pascal_case(field),
            Self::Camel => camel_case(field),
            Self::Kebab => field.replace('_', "-"),
            Self::ScreamingKebab => field.to_ascii_uppercase().replace('_', "-"),
        }
    }
}

/// Convert a field to camel case while preserving empty and one-byte names.
fn camel_case(field: &str) -> String {
    // Derive camel case from Pascal case while preserving an empty field.
    let pascal = pascal_case(field);
    let Some(first) = pascal.get(0..1) else {
        return pascal;
    };
    // Preserve the original Pascal spelling when no remainder follows the first byte.
    let Some(rest) = pascal.get(1..) else {
        return pascal;
    };
    first.to_ascii_lowercase() + rest
}

impl EarlyLintPass for SerdeManualRenameAll {
    /// Check all cfg-active Serde structs in the crate.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        check_crate(cx, krate);
    }
}

/// Check structs for one common explicit field rename convention.
fn check_crate(cx: &EarlyContext<'_>, krate: &Crate) {
    // Collect cfg-active Serde structs before comparing their explicit names.
    let krate = serde_ast_crate(cx, krate);

    for item in &krate.items {
        check_item(cx, item);
    }
}

/// Check one cfg-active struct for a common explicit rename convention.
fn check_item(cx: &EarlyContext<'_>, item: &AstItemInfo<'_>) {
    // Keep only multi-field Serde structs with named fields and no container rule.
    if item.kind != ItemKind::Struct || !item.derives.has_serde() {
        return;
    }
    if !has_renamable_fields(item) || ast_has_serde_attr(cx, item.attrs, "rename_all") {
        return;
    }

    // Each active direction must be fully representable before field attributes can go away.
    let serialize_rule = item
        .derives
        .has_serialize
        .then(|| common_rule(cx, &item.fields, Direction::Serialize))
        .flatten();
    let deserialize_rule = item
        .derives
        .has_deserialize
        .then(|| common_rule(cx, &item.fields, Direction::Deserialize))
        .flatten();
    if (item.derives.has_serialize && serialize_rule.is_none())
        || (item.derives.has_deserialize && deserialize_rule.is_none())
    {
        return;
    }

    // Anchor the consolidated suggestion at the first field-level rename.
    let Some(first_attr) = item
        .fields
        .iter()
        .find_map(|field| ast_serde_attr(cx, field.attrs, "rename"))
    else {
        return;
    };
    emit_lint(cx, first_attr.span, serialize_rule, deserialize_rule);
}

/// Return whether a struct has enough named fields for a container rule.
fn has_renamable_fields(item: &AstItemInfo<'_>) -> bool {
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
fn common_rule(
    cx: &EarlyContext<'_>,
    fields: &[AstFieldInfo<'_>],
    direction: Direction,
) -> Option<RenameRule> {
    RenameRule::ALL.into_iter().find(|rule| {
        fields.iter().all(|field| {
            let Some(name) = field.name.as_deref() else {
                return false;
            };
            if !name.is_ascii() {
                return false;
            }
            let Some(rename) = ast_serde_directional_value(cx, field.attrs, "rename") else {
                return false;
            };
            let explicit = match direction {
                Direction::Serialize => rename.serialize.as_deref(),
                Direction::Deserialize => rename.deserialize.as_deref(),
            };

            explicit.is_some_and(|explicit| rule.apply(name) == explicit)
        })
    })
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

/// Emit the common container-level replacement.
fn emit_lint(
    cx: &EarlyContext<'_>,
    span: rustc_span::Span,
    serialize: Option<RenameRule>,
    deserialize: Option<RenameRule>,
) {
    // Select the shortest container attribute that preserves both active directions.
    let replacement = match (serialize, deserialize) {
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
        // Single-direction derives retain only their active rename rule.
        (Some(serialize), None) => {
            format!("#[serde(rename_all(serialize = \"{}\"))]", serialize.name())
        }
        (None, Some(deserialize)) => format!(
            "#[serde(rename_all(deserialize = \"{}\"))]",
            deserialize.name()
        ),
        (None, None) => return,
    };

    // Suggest the shared attribute while naming the field attributes it replaces.
    cx.emit_span_lint(
        SERDE_MANUAL_RENAME_ALL,
        span,
        DiagDecorator(move |diagnostic| {
            let _configured_diagnostic = diagnostic
                .primary_message("all fields use one Serde rename convention")
                .help(format!(
                    "add `{replacement}` to the struct and remove its field-level renames"
                ));
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
