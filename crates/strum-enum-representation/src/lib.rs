#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks enum string representations that should use Strum or diverge from Serde.
//!
//! The pass finds fieldless enums that hand-write both `Display` and `FromStr`,
//! and enums whose derived Serde and Strum implementations produce or accept
//! different names. Derives are resolved from the implementations the derive
//! macros generate, so items are matched by definition rather than by name.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use {serde as _, strum as _};

use std::collections::{BTreeSet, HashMap};

use heck::{
    ToKebabCase, ToLowerCamelCase, ToShoutySnakeCase, ToSnakeCase, ToTitleCase, ToTrainCase,
    ToUpperCamelCase,
};
use rustc_hir::{Item, ItemKind, VariantData};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty;
use rustc_span::{
    def_id::{DefId, LocalDefId},
    sym,
};

use serde_support::{
    AdtKind, Help, SerdeItem, SerdeVariant, derive_macro, emit_lint, has_serde_attr,
    impl_self_local_adt, namespace_has_word, namespace_string_values, serde_directional_value,
    serde_item,
};

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub STRUM_ENUM_REPRESENTATION,
    Warn,
    "enum string representations should use Strum and agree with Serde",
    StrumEnumRepresentation,
    StrumEnumRepresentation::default()
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
///
/// The index is built once per crate and then consulted for each fieldless
/// enum, together with the derived Serde and Strum representations.
#[derive(Debug, Default)]
pub struct StrumEnumRepresentation {
    /// Manual conversion traits keyed by the target type.
    manual_traits: HashMap<LocalDefId, ManualTraits>,
}

impl<'tcx> LateLintPass<'tcx> for StrumEnumRepresentation {
    /// Index source-written `Display` and `FromStr` implementations once per crate.
    fn check_crate(&mut self, cx: &LateContext<'tcx>) {
        self.manual_traits = manual_conversion_traits(cx);
    }

    /// Check one fieldless enum for manual conversions and divergent derived names.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Restrict both checks to fieldless enum definitions.
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

        // The two standard impls duplicate the same closed mapping that Strum derives own.
        let traits = self
            .manual_traits
            .get(&item.owner_id.def_id)
            .copied()
            .unwrap_or_default();
        if traits.has_display && traits.has_from_str {
            cx.emit_span_lint(
                STRUM_ENUM_REPRESENTATION,
                ident.span,
                rustc_errors::DiagDecorator(|diagnostic| {
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

        if let Some(item) = serde_item(cx, item) {
            check_derived_representations(cx, &item);
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

/// Determine which representation directions both derive systems implement.
fn representation_modes(cx: &LateContext<'_>, item: &SerdeItem<'_>) -> RepresentationModes {
    // Resolve Strum derives only for enums that derive a Serde direction.
    if !item.derives.has_serde() {
        return RepresentationModes {
            has_output_comparison: false,
            has_input_comparison: false,
        };
    }
    // Compare a direction only when both derive systems implement it.
    let derives = strum_derives(cx, item.def_id);
    let has_strum_output = ["AsRefStr", "Display", "IntoStaticStr", "VariantNames"]
        .iter()
        .any(|name| derives.contains(*name));
    let has_strum_input = derives.contains("EnumString");

    RepresentationModes {
        has_output_comparison: item.derives.has_serialize && has_strum_output,
        has_input_comparison: item.derives.has_deserialize && has_strum_input,
    }
}

/// Return the names of Strum derive macros applied to a local enum.
///
/// A derive is found through the implementations it generates. `IntoStaticStr`
/// implements `From<Enum> for &'static str`, so the enum may appear in the trait
/// arguments rather than as the self type.
fn strum_derives(cx: &LateContext<'_>, adt: LocalDefId) -> BTreeSet<String> {
    // Visit every local trait impl, because a derive can implement any trait.
    let mut names = BTreeSet::new();
    for impls in cx.tcx.all_local_trait_impls(()).values() {
        for &impl_def_id in impls {
            // Skip hand-written impls, which no derive macro generated.
            let impl_def_id = impl_def_id.to_def_id();
            let Some(macro_def_id) = derive_macro(cx, impl_def_id) else {
                continue;
            };
            // Keep Strum derives that generated an impl for this enum.
            let is_strum = cx.tcx.crate_name(macro_def_id.krate).as_str() == "strum_macros";
            if is_strum && is_impl_for_adt(cx, impl_def_id, adt) {
                let _ = names.insert(cx.tcx.item_name(macro_def_id).to_string());
            }
        }
    }
    names
}

/// Return whether an implementation's self type or trait arguments name the ADT.
fn is_impl_for_adt(cx: &LateContext<'_>, impl_def_id: DefId, adt: LocalDefId) -> bool {
    cx.tcx
        .impl_trait_ref(impl_def_id)
        .skip_binder()
        .args
        .types()
        .any(|ty| matches!(ty.peel_refs().kind(), ty::Adt(def, _) if def.did() == adt.to_def_id()))
}

/// Compare variants until the first observable mismatch for this enum.
fn check_derived_representations(cx: &LateContext<'_>, item: &SerdeItem<'_>) {
    if item.kind != AdtKind::Enum {
        return;
    }
    let modes = representation_modes(cx, item);
    if !modes.has_output_comparison && !modes.has_input_comparison {
        return;
    }

    // Resolve container-level rename rules once before walking variants.
    let rules = representation_rules(item);

    // Compare the canonical outbound representation before accepted inputs.
    for variant in &item.variants {
        if modes.has_output_comparison
            && let Some((strum, serde)) = output_mismatch(variant, &rules)
        {
            emit_output_divergence(cx, item, variant, strum.as_deref(), serde.as_deref());
            return;
        }

        // Compare accepted inbound spellings only when both derives implement them.
        if modes.has_input_comparison && has_input_mismatch(item, variant, &rules) {
            emit_input_divergence(cx, item, variant);
            return;
        }
    }
}

/// Rules extracted from container-level Serde and Strum attributes.
#[derive(Clone, Debug, Default)]
struct RepresentationRules {
    /// Serde's `rename_all` rule for serialization.
    serde_output: Option<String>,
    /// Serde's `rename_all` rule for deserialization.
    serde_input: Option<String>,
    /// Strum's `serialize_all` rule.
    strum: Option<String>,
    /// Strum's container prefix.
    prefix: Option<String>,
    /// Strum's container suffix.
    suffix: Option<String>,
}

/// Resolve container-level Serde and Strum representation rules.
fn representation_rules(item: &SerdeItem<'_>) -> RepresentationRules {
    // Keep Serde's directional rename values separate for output and input.
    let serde_rule = serde_directional_value(item.attrs, "rename_all").unwrap_or_default();
    let strum_value = |key| namespace_string_values(item.attrs, "strum", key).next();

    RepresentationRules {
        serde_output: serde_rule.serialize,
        serde_input: serde_rule.deserialize,
        strum: strum_value("serialize_all"),
        prefix: strum_value("prefix"),
        suffix: strum_value("suffix"),
    }
}

/// Output names retained when one variant diverges.
type OutputMismatch = (Option<String>, Option<String>);

/// Return the effective output names when one variant diverges.
fn output_mismatch(
    variant: &SerdeVariant<'_>,
    rules: &RepresentationRules,
) -> Option<OutputMismatch> {
    // Resolve each derive's canonical name with its own attribute precedence.
    let serde = serde_output_name(variant, rules.serde_output.as_deref());
    let strum = strum_output_name(variant, rules);

    // Preserve both names for the diagnostic when the effective contracts differ.
    (serde != strum).then_some((strum, serde))
}

/// Return whether one variant accepts different input spellings.
fn has_input_mismatch(
    item: &SerdeItem<'_>,
    variant: &SerdeVariant<'_>,
    rules: &RepresentationRules,
) -> bool {
    // Compare complete accepted-name sets so aliases and catch-all variants remain visible.
    serde_input_names(variant, rules.serde_input.as_deref())
        != strum_input_names(item, variant, rules.strum.as_deref())
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
fn serde_output_name(variant: &SerdeVariant<'_>, container_rule: Option<&str>) -> Option<String> {
    // Treat Serde direction skips as an absent outbound representation.
    if has_serde_attr(variant.attrs, "skip") || has_serde_attr(variant.attrs, "skip_serializing") {
        return None;
    }

    let rename = serde_directional_value(variant.attrs, "rename").unwrap_or_default();
    rename
        .serialize
        .or_else(|| serde_variant_case(variant.ident.name.as_str(), container_rule))
}

/// Return Strum's canonical output name for one variant.
fn strum_output_name(variant: &SerdeVariant<'_>, rules: &RepresentationRules) -> Option<String> {
    // Apply Strum's documented preference order only to enabled variants.
    if namespace_has_word(variant.attrs, "strum", "disabled") {
        return None;
    }

    // Read explicit variant spellings before applying the container case rule.
    let to_string = namespace_string_values(variant.attrs, "strum", "to_string").next();
    let serializations = namespace_string_values(variant.attrs, "strum", "serialize");

    // Select Strum's longest explicit serialization when no `to_string` wins.
    let preferred = to_string
        .or_else(|| serializations.max_by_key(String::len))
        .or_else(|| strum_variant_case(variant.ident.name.as_str(), rules.strum.as_deref()))?;

    // Apply container-level affixes after selecting the effective variant name.
    let prefix = rules.prefix.as_deref().unwrap_or_default();
    let suffix = rules.suffix.as_deref().unwrap_or_default();
    Some(format!("{prefix}{preferred}{suffix}"))
}

/// Return Serde's accepted deserialization names for one variant.
fn serde_input_names(
    variant: &SerdeVariant<'_>,
    container_rule: Option<&str>,
) -> Option<AcceptedNames> {
    // Model skip and catch-all behavior before collecting exact accepted names.
    if has_serde_attr(variant.attrs, "skip") || has_serde_attr(variant.attrs, "skip_deserializing")
    {
        return None;
    }
    if has_serde_attr(variant.attrs, "other") {
        return Some(AcceptedNames::Any);
    }

    // Resolve the primary spelling from directional rename or the container rule.
    let rename = serde_directional_value(variant.attrs, "rename").unwrap_or_default();
    let primary = rename
        .deserialize
        .or_else(|| serde_variant_case(variant.ident.name.as_str(), container_rule))?;
    let mut names = BTreeSet::from([primary]);
    // Add every alias because Serde accepts aliases in addition to the primary spelling.
    names.extend(namespace_string_values(variant.attrs, "serde", "alias"));

    Some(AcceptedNames::Exact(names))
}

/// Return Strum's accepted `EnumString` names for one variant.
fn strum_input_names(
    item: &SerdeItem<'_>,
    variant: &SerdeVariant<'_>,
    container_rule: Option<&str>,
) -> Option<AcceptedNames> {
    // Preserve Strum's disabled, catch-all, and case-insensitive input states distinctly.
    if namespace_has_word(variant.attrs, "strum", "disabled") {
        return None;
    }
    if namespace_has_word(variant.attrs, "strum", "default") {
        return Some(AcceptedNames::Any);
    }

    // Collect explicit serializations before applying the fallback case rule.
    let mut names = BTreeSet::new();
    names.extend(namespace_string_values(variant.attrs, "strum", "serialize"));
    names.extend(namespace_string_values(variant.attrs, "strum", "to_string"));
    if names.is_empty() {
        let _ = names.insert(strum_variant_case(
            variant.ident.name.as_str(),
            container_rule,
        )?);
    }

    // Preserve Strum's case-insensitive mode as a distinct accepted-name set.
    let is_case_insensitive = namespace_has_word(item.attrs, "strum", "ascii_case_insensitive")
        || namespace_has_word(variant.attrs, "strum", "ascii_case_insensitive");
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

    // Keep the no-rule case local and apply named compatibility groups in order.
    rule.map_or_else(
        || Some(name.to_owned()),
        |rule| {
            // Check Heck's camel-case spellings before separator and simple cases.
            strum_camel_case(name, rule)
                // Check snake and kebab aliases as one separator group.
                .or_else(|| strum_separator_case(name, rule))
                // Check shouting variants after ordinary separators.
                .or_else(|| strum_shouting_case(name, rule))
                // Check the remaining direct casing rules last.
                .or_else(|| strum_simple_case(name, rule))
        },
    )
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

/// Emit an output-name mismatch with both concrete spellings.
fn emit_output_divergence(
    cx: &LateContext<'_>,
    item: &SerdeItem<'_>,
    variant: &SerdeVariant<'_>,
    strum: Option<&str>,
    serde: Option<&str>,
) {
    // Render absent representations explicitly so skipped directions remain diagnosable.
    let enum_name = item.ident;
    let variant_name = variant.ident;
    let strum_name = displayed_name(strum);
    let serde_name = displayed_name(serde);

    // Name both spellings so the mismatch is actionable without expanding macros.

    emit_divergence(
        cx,
        variant,
        format!(
            "enum `{enum_name}` has divergent Strum and Serde output for variant `{variant_name}`: Strum uses {strum_name}; Serde uses {serde_name}",
        ),
    );
}

/// Emit an accepted-input mismatch for one variant.
fn emit_input_divergence(cx: &LateContext<'_>, item: &SerdeItem<'_>, variant: &SerdeVariant<'_>) {
    // Input sets can include aliases or catch-all behavior, so identify the direction explicitly.
    let enum_name = item.ident;
    let variant_name = variant.ident;
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
fn emit_divergence(cx: &LateContext<'_>, variant: &SerdeVariant<'_>, message: String) {
    emit_lint(
        cx,
        STRUM_ENUM_REPRESENTATION,
        variant.hir_id,
        variant.span,
        message,
        Help::text("align the Strum and Serde representation attributes"),
    );
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
            // Macro-generated impls, including derives, are not hand-written mappings.
            if cx.tcx.def_span(impl_def_id).from_expansion() {
                continue;
            }
            let Some(local_type) = impl_self_local_adt(cx, impl_def_id.to_def_id()) else {
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
fn conversion_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> Option<ConversionTrait> {
    if cx.tcx.is_diagnostic_item(sym::Display, trait_def_id) {
        return Some(ConversionTrait::Display);
    }
    // `FromStr` has no diagnostic item, so match its defining crate and name.
    (cx.tcx.crate_name(trait_def_id.krate) == sym::core
        && cx.tcx.item_name(trait_def_id).as_str() == "FromStr")
        .then_some(ConversionTrait::FromStr)
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
