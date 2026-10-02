#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks enum string representations that should use Strum or diverge from Serde.
//! The early pass compares derive attributes and effective names before macro
//! expansion removes helper metadata. The late pass finds fieldless enums that
//! hand-write both standard conversion directions instead of deriving them.
//! Shared Serde and Strum support code keeps output and input comparisons
//! separate, preserves aliases and catch-all variants, and reports the first
//! source-authored mismatch with concrete guidance.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

#[cfg(test)]
use {serde as _, strum as _};

use std::collections::{BTreeSet, HashMap};

use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase, ToTrainCase,
    ToUpperCamelCase,
};
use rustc_ast::{Attribute, Crate, LitKind, MetaItemKind};
use rustc_errors::DiagDecorator;
use rustc_hir::{Item, ItemKind, VariantData};
use rustc_lint::{EarlyContext, EarlyLintPass, LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{Symbol, def_id::LocalDefId, sym};

use serde_support::{
    AstItemInfo, AstVariantInfo, ItemKind as SerdeItemKind, ast_has_serde_attr,
    ast_serde_directional_value, loaded_rust_sources, parse_items, serde_ast_crate,
};

dylint_linting::__maybe_exclude! {
    dylint_linting::dylint_library!();
}

rustc_session::declare_lint! {
    #[doc = include_str!("../README.md")]
    pub STRUM_ENUM_REPRESENTATION,
    Warn,
    "enum string representations should use Strum and agree with Serde"
}

dylint_linting::__maybe_mangle! {
    /// Register the early attribute pass and late manual-conversion pass.
    ///
    /// The caller supplies rustc's session and lint store during library
    /// initialization. Registration installs both passes under one lint
    /// declaration; source inspection begins when rustc invokes each pass.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #![feature(rustc_private)]
    /// let _call = |sess, lint_store| {
    ///     let _ = strum_enum_representation::register_lints(sess, lint_store);
    /// };
    /// ```
    pub fn register_lints(
        sess: &rustc_session::Session,
        lint_store: &mut rustc_lint::LintStore,
    ) {
        // Register one lint declaration before attaching its early and late evidence passes.
        dylint_linting::init_config(sess);
        lint_store.register_lints(&[STRUM_ENUM_REPRESENTATION]);
        lint_store.register_early_lint_pass(Box::new(|| Box::new(StrumSerdeRepresentation)));
        lint_store.register_late_lint_pass(
            Box::new(dylint_linting::__make_late_closure!(StrumEnumRepresentation::default())),
        );
    }
}

/// Early pass that compares derive-helper attributes before macro expansion
/// removes them.
#[derive(Clone, Copy, Debug, Default)]
struct StrumSerdeRepresentation;

impl rustc_lint::LintPass for StrumSerdeRepresentation {
    fn name(&self) -> &'static str {
        "StrumSerdeRepresentation"
    }

    fn get_lints(&self) -> rustc_lint::LintVec {
        vec![STRUM_ENUM_REPRESENTATION]
    }
}

impl rustc_lint::LintPass for StrumEnumRepresentation {
    fn name(&self) -> &'static str {
        "StrumEnumRepresentation"
    }

    fn get_lints(&self) -> rustc_lint::LintVec {
        vec![STRUM_ENUM_REPRESENTATION]
    }
}

impl EarlyLintPass for StrumSerdeRepresentation {
    /// Compare cfg-active fieldless enums that derive both representation systems.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Retain cfg-active Serde items before recovering Strum derives from source.
        let krate = serde_ast_crate(cx, krate);

        // Filter the source inventory before inspecting representation directions.
        for item in &krate.items {
            if item.kind != SerdeItemKind::Enum
                || !item.variants.iter().all(|variant| variant.is_unit)
            {
                continue;
            }

            // Keep output and input evidence independent for one enum.
            let modes = representation_modes(cx, item);
            if !modes.has_output_comparison() && !modes.has_input_comparison() {
                continue;
            }

            check_derived_representations(cx, item, modes);
        }
    }
}

/// Derived directions available from both Serde and Strum.
#[derive(Clone, Copy, Debug)]
struct RepresentationModes {
    /// Whether to compare `Serialize` with a Strum output derive.
    has_output_comparison: bool,
    /// Whether to compare `Deserialize` with `EnumString`.
    has_input_comparison: bool,
}

impl RepresentationModes {
    /// Return whether both crates emit an outbound representation.
    const fn has_output_comparison(self) -> bool {
        self.has_output_comparison
    }

    /// Return whether both crates accept an inbound representation.
    const fn has_input_comparison(self) -> bool {
        self.has_input_comparison
    }
}

/// Determine which representation directions both derive systems implement.
fn representation_modes(cx: &EarlyContext<'_>, item: &AstItemInfo<'_>) -> RepresentationModes {
    // Keep output and input derivations independent because each crate supports one-way derives.
    let derives = derive_names(cx, item.attrs, &item.name);
    let has_strum_output = ["AsRefStr", "Display", "IntoStaticStr", "VariantNames"]
        .iter()
        .any(|name| derives.contains(*name));
    let has_strum_input = derives.contains("EnumString");

    RepresentationModes {
        has_output_comparison: item.derives.has_serialize && has_strum_output,
        has_input_comparison: item.derives.has_deserialize && has_strum_input,
    }
}

/// Compare variants until the first observable mismatch for this enum.
fn check_derived_representations(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    modes: RepresentationModes,
) {
    // Resolve container-level rename rules once before walking variants.
    let (serde_output_rule, serde_input_rule, strum_rule, prefix, suffix) =
        representation_rules(cx, item);

    // Compare the canonical outbound representation before accepted inputs.
    for variant in &item.variants {
        if modes.has_output_comparison()
            && let Some((strum, serde)) = output_mismatch(
                cx,
                variant,
                serde_output_rule.as_deref(),
                strum_rule.as_deref(),
                prefix.as_deref(),
                suffix.as_deref(),
            )
        {
            emit_output_divergence(cx, item, variant, strum.as_deref(), serde.as_deref());
            return;
        }

        // Compare accepted inbound spellings only when both derives implement them.
        if modes.has_input_comparison()
            && has_input_mismatch(
                cx,
                item,
                variant,
                serde_input_rule.as_deref(),
                strum_rule.as_deref(),
            )
        {
            emit_input_divergence(cx, item, variant);
            return;
        }
    }
}

/// Rules extracted from container-level Serde and Strum attributes.
type RepresentationRules = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

/// Resolve container-level Serde and Strum representation rules.
fn representation_rules(cx: &EarlyContext<'_>, item: &AstItemInfo<'_>) -> RepresentationRules {
    // Keep Serde's directional rename values separate for output and input.
    let serde_rule = ast_serde_directional_value(cx, item.attrs, "rename_all").unwrap_or_default();
    let serde_output = serde_rule.serialize;
    let serde_input = serde_rule.deserialize;

    // Read Strum's shared case and affix settings once for every variant.
    let strum_rule = attribute_string_values(cx, item.attrs, "strum", "serialize_all")
        .into_iter()
        .next();
    let prefix = attribute_string_values(cx, item.attrs, "strum", "prefix")
        .into_iter()
        .next();
    let suffix = attribute_string_values(cx, item.attrs, "strum", "suffix")
        .into_iter()
        .next();

    (serde_output, serde_input, strum_rule, prefix, suffix)
}

/// Output names retained when one variant diverges.
type OutputMismatch = (Option<String>, Option<String>);

/// Return the effective output names when one variant diverges.
fn output_mismatch(
    cx: &EarlyContext<'_>,
    variant: &AstVariantInfo<'_>,
    serde_rule: Option<&str>,
    strum_rule: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
) -> Option<OutputMismatch> {
    // Resolve each derive's canonical name with its own attribute precedence.
    let serde = serde_output_name(cx, variant, serde_rule);
    let strum = strum_output_name(cx, variant, strum_rule, prefix, suffix);

    // Preserve both names for the diagnostic when the effective contracts differ.
    (serde != strum).then_some((strum, serde))
}

/// Return whether one variant accepts different input spellings.
fn has_input_mismatch(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    variant: &AstVariantInfo<'_>,
    serde_rule: Option<&str>,
    strum_rule: Option<&str>,
) -> bool {
    // Compare complete accepted-name sets so aliases and catch-all variants remain visible.
    serde_input_names(cx, variant, serde_rule) != strum_input_names(cx, item, variant, strum_rule)
}

/// Accepted input spellings for one enum variant.
#[derive(Clone, Debug, Eq, PartialEq)]
enum AcceptedNames {
    /// One closed set of case-sensitive spellings.
    Exact(BTreeSet<String>),
    /// One closed set matched without ASCII case distinctions.
    AsciiCaseInsensitive(BTreeSet<String>),
    /// Every otherwise unmatched spelling maps to this variant.
    Any,
}

/// Return Serde's canonical serialization name for one variant.
fn serde_output_name(
    cx: &EarlyContext<'_>,
    variant: &AstVariantInfo<'_>,
    container_rule: Option<&str>,
) -> Option<String> {
    // Treat Serde direction skips as an absent outbound representation.
    if ast_has_serde_attr(cx, variant.attrs, "skip")
        || ast_has_serde_attr(cx, variant.attrs, "skip_serializing")
    {
        return None;
    }

    let rename = ast_serde_directional_value(cx, variant.attrs, "rename").unwrap_or_default();
    rename
        .serialize
        .or_else(|| serde_variant_case(&variant.name, container_rule))
}

/// Return Strum's canonical output name for one variant.
fn strum_output_name(
    cx: &EarlyContext<'_>,
    variant: &AstVariantInfo<'_>,
    container_rule: Option<&str>,
    prefix: Option<&str>,
    suffix: Option<&str>,
) -> Option<String> {
    // Apply Strum's documented preference order only to enabled variants.
    if attribute_has_word(cx, variant.attrs, "strum", "disabled") {
        return None;
    }

    // Read explicit variant spellings before applying the container case rule.
    let to_string = attribute_string_values(cx, variant.attrs, "strum", "to_string")
        .into_iter()
        .next();
    let serializations = attribute_string_values(cx, variant.attrs, "strum", "serialize");

    // Select Strum's longest explicit serialization when no `to_string` wins.
    let preferred = to_string
        .or_else(|| serializations.into_iter().max_by_key(String::len))
        .or_else(|| strum_variant_case(&variant.name, container_rule))?;

    // Apply container-level affixes after selecting the effective variant name.
    let prefix = prefix.unwrap_or_default();
    let suffix = suffix.unwrap_or_default();
    Some(format!("{prefix}{preferred}{suffix}"))
}

/// Return Serde's accepted deserialization names for one variant.
fn serde_input_names(
    cx: &EarlyContext<'_>,
    variant: &AstVariantInfo<'_>,
    container_rule: Option<&str>,
) -> Option<AcceptedNames> {
    // Model skip and catch-all behavior before collecting exact accepted names.
    if ast_has_serde_attr(cx, variant.attrs, "skip")
        || ast_has_serde_attr(cx, variant.attrs, "skip_deserializing")
    {
        return None;
    }
    if ast_has_serde_attr(cx, variant.attrs, "other") {
        return Some(AcceptedNames::Any);
    }

    // Resolve the primary spelling from directional rename or the container rule.
    let rename = ast_serde_directional_value(cx, variant.attrs, "rename").unwrap_or_default();
    let primary = rename
        .deserialize
        .or_else(|| serde_variant_case(&variant.name, container_rule))?;
    let mut names = BTreeSet::from([primary]);
    // Add every alias because Serde accepts aliases in addition to the primary spelling.
    names.extend(attribute_string_values(cx, variant.attrs, "serde", "alias"));

    Some(AcceptedNames::Exact(names))
}

/// Return Strum's accepted `EnumString` names for one variant.
fn strum_input_names(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    variant: &AstVariantInfo<'_>,
    container_rule: Option<&str>,
) -> Option<AcceptedNames> {
    // Preserve Strum's disabled, catch-all, and case-insensitive input states distinctly.
    if attribute_has_word(cx, variant.attrs, "strum", "disabled") {
        return None;
    }
    if attribute_has_word(cx, variant.attrs, "strum", "default") {
        return Some(AcceptedNames::Any);
    }

    // Collect explicit serializations before applying the fallback case rule.
    let mut names = BTreeSet::new();
    names.extend(attribute_string_values(
        cx,
        variant.attrs,
        "strum",
        "serialize",
    ));
    names.extend(attribute_string_values(
        cx,
        variant.attrs,
        "strum",
        "to_string",
    ));
    if names.is_empty() {
        let _ = names.insert(strum_variant_case(&variant.name, container_rule)?);
    }

    // Preserve Strum's case-insensitive mode as a distinct accepted-name set.
    let is_case_insensitive = attribute_has_word(cx, item.attrs, "strum", "ascii_case_insensitive")
        || attribute_has_word(cx, variant.attrs, "strum", "ascii_case_insensitive");
    Some(if is_case_insensitive {
        AcceptedNames::AsciiCaseInsensitive(names)
    } else {
        AcceptedNames::Exact(names)
    })
}

/// Apply Serde 1.0.228's enum-variant case conversion.
fn serde_variant_case(name: &str, rule: Option<&str>) -> Option<String> {
    // The local profile deliberately avoids guessing across Unicode case-conversion differences.
    if !name.is_ascii() {
        return None;
    }

    // Keep direct case conversions separate from separator-based conversions.
    match rule {
        None | Some("PascalCase") => Some(name.to_owned()),
        Some("lowercase") => Some(name.to_ascii_lowercase()),
        Some("UPPERCASE") => Some(name.to_ascii_uppercase()),
        Some("camelCase") => Some(lowercase_first_ascii(name)),
        Some(rule) => serde_separator_case(name, rule),
    }
}

/// Apply Serde's separator-based enum case conversions.
fn serde_separator_case(name: &str, rule: &str) -> Option<String> {
    let requested_rule = rule;
    let rule = SERDE_SEPARATOR_RULES
        .iter()
        .find_map(|(candidate, typed_rule)| {
            (*candidate == requested_rule).then_some(*typed_rule)
        })?;

    // Build the shared snake-case spelling once for all separator variants.
    let snake = serde_snake_case(name);

    // Preserve Serde's exact separator and casing vocabulary.
    match rule {
        SerdeSeparatorRule::Snake => Some(snake),
        SerdeSeparatorRule::ScreamingSnake => Some(snake.to_ascii_uppercase()),
        SerdeSeparatorRule::Kebab => Some(snake.replace('_', "-")),
        SerdeSeparatorRule::ScreamingKebab => Some(snake.to_ascii_uppercase().replace('_', "-")),
    }
}

/// Closed Serde separator rules accepted by this lint's local profile.
#[derive(Clone, Copy, Debug)]
enum SerdeSeparatorRule {
    /// Lowercase words separated with underscores.
    Snake,
    /// Uppercase words separated with underscores.
    ScreamingSnake,
    /// Lowercase words separated with hyphens.
    Kebab,
    /// Uppercase words separated with hyphens.
    ScreamingKebab,
}

/// Map Serde's external rule spellings to typed separator rules.
const SERDE_SEPARATOR_RULES: [(&str, SerdeSeparatorRule); 4] = [
    ("snake_case", SerdeSeparatorRule::Snake),
    ("SCREAMING_SNAKE_CASE", SerdeSeparatorRule::ScreamingSnake),
    ("kebab-case", SerdeSeparatorRule::Kebab),
    ("SCREAMING-KEBAB-CASE", SerdeSeparatorRule::ScreamingKebab),
];

/// Apply Strum 0.28's documented `serialize_all` case conversion.
fn strum_variant_case(name: &str, rule: Option<&str>) -> Option<String> {
    // Mirror Strum 0.28's Heck-backed rules and its documented compatibility aliases.
    if !name.is_ascii() {
        return None;
    }

    // Keep the no-rule case local and delegate named compatibility groups.
    rule.map_or_else(
        || Some(name.to_owned()),
        |rule| strum_named_case(name, rule),
    )
}

/// Apply one named Strum case rule by checking independent compatibility groups.
fn strum_named_case(name: &str, rule: &str) -> Option<String> {
    // Check Heck's camel-case spellings before separator and simple cases.
    strum_camel_case(name, rule)
        // Check snake and kebab aliases as one separator group.
        .or_else(|| strum_separator_case(name, rule))
        // Check shouting variants after ordinary separators.
        .or_else(|| strum_shouting_case(name, rule))
        // Check the remaining direct casing rules last.
        .or_else(|| strum_simple_case(name, rule))
}

/// Apply Strum's camel-case compatibility rules.
fn strum_camel_case(name: &str, rule: &str) -> Option<String> {
    match rule {
        "PascalCase" | "camel_case" => Some(name.to_upper_camel_case()),
        "camelCase" => Some(lowercase_first_ascii(&name.to_upper_camel_case())),
        _ => None,
    }
}

/// Apply Strum's snake-case and kebab-case compatibility rules.
fn strum_separator_case(name: &str, rule: &str) -> Option<String> {
    match rule {
        "snake_case" | "snek_case" => Some(name.to_snake_case()),
        "kebab-case" | "kebab_case" => Some(name.to_kebab_case()),
        _ => None,
    }
}

/// Apply Strum's shouting case compatibility rules.
fn strum_shouting_case(name: &str, rule: &str) -> Option<String> {
    match rule {
        "SCREAMING_SNAKE_CASE" | "shouty_snake_case" | "shouty_snek_case" => {
            Some(name.to_shouty_snake_case())
        }
        "SCREAMING-KEBAB-CASE" => Some(name.to_kebab_case().to_ascii_uppercase()),
        _ => None,
    }
}

/// Apply Strum's remaining direct casing rules.
fn strum_simple_case(name: &str, rule: &str) -> Option<String> {
    let requested_rule = rule;
    let rule = STRUM_SIMPLE_RULES
        .iter()
        .find_map(|(candidate, typed_rule)| {
            (*candidate == requested_rule).then_some(*typed_rule)
        })?;

    match rule {
        StrumSimpleRule::Lowercase => Some(name.to_ascii_lowercase()),
        StrumSimpleRule::Uppercase => Some(name.to_ascii_uppercase()),
        StrumSimpleRule::Title => Some(name.to_title_case()),
        StrumSimpleRule::Mixed => Some(name.to_lower_camel_case()),
        StrumSimpleRule::Train => Some(name.to_train_case()),
    }
}

/// Closed direct Strum case rules accepted by this lint's local profile.
#[derive(Clone, Copy, Debug)]
enum StrumSimpleRule {
    /// Lowercase every output character.
    Lowercase,
    /// Uppercase every output character.
    Uppercase,
    /// Capitalize each title word.
    Title,
    /// Use lower camel case.
    Mixed,
    /// Use title words separated by hyphens.
    Train,
}

/// Map Strum's external rule spellings to typed direct case rules.
const STRUM_SIMPLE_RULES: [(&str, StrumSimpleRule); 5] = [
    ("lowercase", StrumSimpleRule::Lowercase),
    ("UPPERCASE", StrumSimpleRule::Uppercase),
    ("title_case", StrumSimpleRule::Title),
    ("mixed_case", StrumSimpleRule::Mixed),
    ("Train-Case", StrumSimpleRule::Train),
];

/// Lowercase the first ASCII byte of a nonempty Rust identifier.
fn lowercase_first_ascii(name: &str) -> String {
    // Lowercase only the leading character while preserving the remaining identifier spelling.
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };

    // Reserve the source length because ASCII case conversion preserves byte width.
    let mut converted = String::with_capacity(name.len());
    converted.push(first.to_ascii_lowercase());
    converted.extend(characters);
    converted
}

/// Apply Serde's uppercase-boundary insertion before lowercasing a variant.
fn serde_snake_case(name: &str) -> String {
    // Serde inserts a boundary before every non-leading uppercase character.
    let mut snake = String::new();

    // Walk source character boundaries so acronym transitions remain deterministic.
    for (index, character) in name.char_indices() {
        if index > 0 && character.is_ascii_uppercase() {
            snake.push('_');
        }
        snake.push(character.to_ascii_lowercase());
    }

    snake
}

/// Return final derive names from the item's structured derive attributes.
fn derive_names(cx: &EarlyContext<'_>, attrs: &[Attribute], item_name: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    // Prefer structured derive metadata while it remains available.
    for attr in attrs.iter().filter(|attr| attr.has_name(sym::derive)) {
        let Some(items) = attr.meta_item_list() else {
            continue;
        };
        for item in items {
            let Some(meta) = item.meta_item() else {
                continue;
            };
            let Some(segment) = meta.path.segments.iter().next_back() else {
                continue;
            };
            let _ = names.insert(segment.ident.name.to_ident_string());
        }

        // Proc-macro derive metadata can be thin, so parse only this bounded derive source.
        if let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span)
            && let Some(arguments) = parenthesized_arguments(&source)
        {
            record_derive_arguments(&mut names, arguments);
        }
    }

    // Merge source recovery because proc-macro expansion can consume derive metadata.
    names.extend(source_derive_names(cx, item_name));

    // Return final path segments so direct and qualified derive names compare alike.
    names
}

/// Recover derive names from local source after macro expansion consumes attributes.
fn source_derive_names(cx: &EarlyContext<'_>, item_name: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    // Derive attributes may be consumed before this early pass; recover only matching source
    // items and merge their final derive path segments.
    for candidate in loaded_rust_sources(cx) {
        for item in parse_items(&candidate.source)
            .into_iter()
            .filter(|item| item.name == item_name)
        {
            for attr in item.attrs {
                let Some(source) = candidate.source.get(attr.start..attr.end) else {
                    continue;
                };
                if !source.trim_start().starts_with("#[derive") {
                    continue;
                }
                let Some(arguments) = parenthesized_arguments(source) else {
                    continue;
                };
                record_derive_arguments(&mut names, arguments);
            }
        }
    }

    // Return the recovered names without retaining source slices beyond this pass.
    names
}

/// Add final derive path segments from one comma-separated argument list.
fn record_derive_arguments(names: &mut BTreeSet<String>, arguments: &str) {
    // Store final path segments so qualified and direct derive spellings compare identically.
    for argument in arguments.split(',') {
        let name = argument.trim().rsplit("::").next().unwrap_or_default();
        if !name.is_empty() {
            let _ = names.insert(name.to_owned());
        }
    }
}

/// Return every string assigned to one key in namespaced attributes.
fn attribute_string_values(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    namespace: &str,
    key: &str,
) -> Vec<String> {
    // Intern the namespace once because every candidate attribute uses the same symbol.
    let namespace = Symbol::intern(namespace);
    let mut values = Vec::new();

    // Inspect only attributes in the requested namespace.
    for attr in attrs.iter().filter(|attr| attr.has_name(namespace)) {
        if let Some(items) = attr.meta_item_list() {
            // Prefer structured literals when rustc has retained helper metadata.
            for item in items {
                let Some(meta) = item.meta_item() else {
                    continue;
                };
                if meta_path_name(meta) != Some(key) {
                    continue;
                }
                let MetaItemKind::NameValue(literal) = &meta.kind else {
                    continue;
                };
                let LitKind::Str(value, _) = literal.kind else {
                    continue;
                };
                values.push(value.to_ident_string());
            }
        }

        // Retain helper attributes even when early structured metadata is unavailable.
        if values.is_empty()
            && let Ok(source) = cx.sess().source_map().span_to_snippet(attr.span)
        {
            values.extend(source_string_assignments(&source, key));
        }
    }

    values
}

/// Return whether a namespaced attribute contains one word entry.
fn attribute_has_word(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    namespace: &str,
    key: &str,
) -> bool {
    // Prefer structured metadata and fall back to the bounded helper-attribute source.
    let namespace = Symbol::intern(namespace);

    attrs
        .iter()
        .filter(|attr| attr.has_name(namespace))
        .any(|attr| {
            let is_structured = attr.meta_item_list().is_some_and(|items| {
                items.iter().any(|item| {
                    item.meta_item().is_some_and(|meta| {
                        meta_path_name(meta) == Some(key) && matches!(meta.kind, MetaItemKind::Word)
                    })
                })
            });
            is_structured
                || cx
                    .sess()
                    .source_map()
                    .span_to_snippet(attr.span)
                    .ok()
                    .and_then(|source| parenthesized_arguments(&source).map(str::to_owned))
                    .is_some_and(|arguments| {
                        arguments.split(',').any(|argument| argument.trim() == key)
                    })
        })
}

/// Return the final path segment of a meta item.
fn meta_path_name(meta: &rustc_ast::MetaItem) -> Option<&str> {
    meta.path
        .segments
        .iter()
        .next_back()
        .map(|segment| segment.ident.name.as_str())
}

/// Return the contents between one attribute's outer parentheses.
fn parenthesized_arguments(source: &str) -> Option<&str> {
    let start = source.find('(')?;
    let end = source.rfind(')')?;
    source.get(start + 1..end)
}

/// Parse repeated `key = "value"` entries from one bounded attribute source.
fn source_string_assignments(source: &str, key: &str) -> Vec<String> {
    // Parse only direct quoted assignments inside one already-bounded helper attribute.
    let Some(arguments) = parenthesized_arguments(source) else {
        return Vec::new();
    };

    arguments
        .split(',')
        .filter_map(|argument| {
            let (candidate, value) = argument.split_once('=')?;
            if candidate.trim() != key {
                return None;
            }
            let value = value.trim();
            value
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect()
}

/// Emit an output-name mismatch with both concrete spellings.
fn emit_output_divergence(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    variant: &AstVariantInfo<'_>,
    strum: Option<&str>,
    serde: Option<&str>,
) {
    // Include both concrete spellings so the mismatch is actionable without macro expansion.
    let enum_name = &item.name;
    let variant_name = &variant.name;

    // Render absent representations explicitly so skipped directions remain diagnosable.
    let strum_name = displayed_name(strum);
    let serde_name = displayed_name(serde);

    // Compose one stable message before emitting the shared diagnostic.
    let message = format!(
        "enum `{enum_name}` has divergent Strum and Serde output for variant `{variant_name}`: Strum uses {strum_name}; Serde uses {serde_name}",
    );
    emit_divergence(cx, variant, message);
}

/// Emit an accepted-input mismatch for one variant.
fn emit_input_divergence(
    cx: &EarlyContext<'_>,
    item: &AstItemInfo<'_>,
    variant: &AstVariantInfo<'_>,
) {
    // Input sets can include aliases or catch-all behavior, so identify the direction explicitly.
    let enum_name = &item.name;
    let variant_name = &variant.name;

    // Keep the message focused on the variant whose accepted names diverge.
    emit_divergence(
        cx,
        variant,
        format!(
            "enum `{enum_name}` has divergent Strum and Serde input names for variant `{variant_name}`"
        ),
    );
}

/// Render one optional output spelling for a diagnostic.
fn displayed_name(name: Option<&str>) -> String {
    name.map_or_else(|| "no name".to_owned(), |name| format!("`{name}`"))
}

/// Emit one representation divergence at the responsible variant.
fn emit_divergence(cx: &EarlyContext<'_>, variant: &AstVariantInfo<'_>, message: String) {
    // Anchor the diagnostic on the first variant whose effective contract differs.
    cx.emit_span_lint(
        STRUM_ENUM_REPRESENTATION,
        variant.span,
        DiagDecorator(move |diagnostic| {
            let _configured_diagnostic = diagnostic
                .primary_message(message)
                .help("align the Strum and Serde representation attributes");
        }),
    );
}

/// Manual conversion traits implemented by one local enum.
#[derive(Clone, Copy, Debug, Default)]
struct ManualTraits {
    /// Whether a source-written `Display` implementation exists.
    has_display: bool,
    /// Whether a source-written `FromStr` implementation exists.
    has_from_str: bool,
}

/// Stateful pass that indexes manual conversion implementations by their local enum.
#[derive(Debug, Default)]
struct StrumEnumRepresentation {
    /// Manual conversion traits keyed by the target type.
    manual_traits: HashMap<LocalDefId, ManualTraits>,
}

impl<'tcx> LateLintPass<'tcx> for StrumEnumRepresentation {
    /// Index source-written `Display` and `FromStr` implementations once per crate.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.manual_traits = manual_conversion_traits(cx);
    }

    /// Warn on fieldless enums that maintain both conversion directions manually.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Restrict this late check to fieldless enum definitions.
        let ItemKind::Enum(ident, _, definition) = item.kind else {
            return;
        };
        if !definition
            .variants
            .iter()
            .all(|variant| matches!(variant.data, VariantData::Unit(..)))
        {
            return;
        }

        // Look up the conversion traits indexed during crate traversal.
        let Some(traits) = self.manual_traits.get(&item.owner_id.def_id) else {
            return;
        };
        // Require both directions before recommending one Strum declaration.
        if !traits.has_display || !traits.has_from_str {
            return;
        }

        // The two standard impls duplicate the same closed mapping that Strum derives own.
        cx.emit_span_lint(
            STRUM_ENUM_REPRESENTATION,
            ident.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message(format!(
                        "fieldless enum `{ident}` hand-writes both `Display` and `FromStr`"
                    ))
                    .help(
                        "derive `strum::Display` and `strum::EnumString` and declare the string mapping once",
                    );
            }),
        );
    }
}

/// Collect manual conversion traits implemented for local ADTs.
fn manual_conversion_traits(cx: &LateContext<'_>) -> HashMap<LocalDefId, ManualTraits> {
    let mut traits_by_type: HashMap<LocalDefId, ManualTraits> = HashMap::new();

    // Resolve trait identities before classifying each local implementation target.
    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        let Some(conversion) = conversion_trait(cx, trait_def_id) else {
            continue;
        };

        // Classify every source-authored implementation for this conversion trait.
        for &impl_def_id in impl_def_ids {
            // Proc-macro-generated impls already use a derive and must not trigger this policy.
            if cx.tcx.def_span(impl_def_id).from_expansion() {
                continue;
            }
            let Some(local_type) = impl_self_local_adt(cx, impl_def_id) else {
                continue;
            };

            // Accumulate both supported traits under the same local ADT.
            let traits = traits_by_type.entry(local_type).or_default();
            match conversion {
                ConversionTrait::Display => traits.has_display = true,
                ConversionTrait::FromStr => traits.has_from_str = true,
            }
        }
    }

    traits_by_type
}

/// Standard conversion trait relevant to a Strum replacement.
#[derive(Clone, Copy, Debug)]
enum ConversionTrait {
    /// `core::fmt::Display`.
    Display,
    /// `core::str::FromStr`.
    FromStr,
}

/// Resolve one trait definition to a supported conversion trait.
fn conversion_trait(
    cx: &LateContext<'_>,
    trait_def_id: rustc_span::def_id::DefId,
) -> Option<ConversionTrait> {
    // Resolve aliases through rustc's trait identity and retain only the two Strum replacements.
    match cx.tcx.def_path_str(trait_def_id).as_str() {
        "core::fmt::Display" | "std::fmt::Display" => Some(ConversionTrait::Display),
        "core::str::traits::FromStr" | "std::str::FromStr" => Some(ConversionTrait::FromStr),
        _ => None,
    }
}

/// Return the local ADT directly targeted by an implementation.
fn impl_self_local_adt(cx: &LateContext<'_>, impl_def_id: LocalDefId) -> Option<LocalDefId> {
    // Avoid querying implementation-only type metadata for non-implementation definitions.
    if !matches!(
        cx.tcx.def_kind(impl_def_id),
        rustc_hir::def::DefKind::Impl { .. }
    ) {
        return None;
    }
    // Resolve the implementation target and retain only local algebraic data types.
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let ty::Adt(adt, _) = self_ty.kind() else {
        return None;
    };

    adt.did().as_local()
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

#[cfg(test)]
mod tests {
    use super::{serde_variant_case, strum_variant_case};

    /// Preserve the upstream acronym difference that makes equal rule names insufficient.
    #[test]
    fn snake_case_rules_diverge_for_acronyms() {
        let serde = serde_variant_case("HTTPResponse", Some("snake_case"));
        let strum = strum_variant_case("HTTPResponse", Some("snake_case"));

        assert_eq!(serde.as_deref(), Some("h_t_t_p_response"));
        assert_eq!(strum.as_deref(), Some("http_response"));
    }
}
