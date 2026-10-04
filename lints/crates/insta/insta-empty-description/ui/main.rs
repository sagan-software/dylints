//! Compiler UI cases for empty Insta snapshot descriptions.
#![feature(rustc_private)]

use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;
use insta_empty_description as _;
use insta_support as _;

/// An empty current-crate description constant.
const EMPTY: &str = "";

/// Exercise literal, constant, and immutable-local descriptions.
fn main() {
    let mut settings = insta::Settings::clone_current();
    settings.set_description("");
    settings.set_description(EMPTY);
    settings.set_description("API response");

    let empty_description = "";
    settings.set_description(empty_description);

    associated_const_description(AssociatedDescription);
    trait_associated_description_controls(DefaultDescription, NamedDescription);
}

/// Provide an empty description through a same-crate associated constant.
struct AssociatedDescription;

impl AssociatedDescription {
    /// Empty description used to test associated-constant resolution.
    const EMPTY: &str = "";
    /// Known nonempty description used as an associated-constant control.
    const NONEMPTY: &str = "API response";
    /// Computed description whose initializer remains unknown to the lint.
    const COMPUTED: &'static str = computed_description();
}

/// Exercise empty, nonempty, computed, and aliased inherent associated constants.
fn associated_const_description(_description: AssociatedDescription) {
    let mut settings = insta::Settings::clone_current();
    settings.set_description(AssociatedDescription::EMPTY);
    settings.set_description(AssociatedDescription::NONEMPTY);
    settings.set_description(AssociatedDescription::COMPUTED);

    let empty_associated_description = AssociatedDescription::EMPTY;
    settings.set_description(empty_associated_description);
}

/// Return an empty value through a computation that the lint cannot inspect.
const fn computed_description() -> &'static str {
    ""
}

/// Supply a default that an implementation may override.
trait DescriptionValue {
    /// Empty trait default; trait-associated values remain unknown.
    const DEFAULT_EMPTY: &'static str = "";
    /// Empty default that a concrete implementation overrides.
    const OVERRIDDEN: &'static str = "";
    /// Computed default whose value is not a direct literal.
    const COMPUTED: &'static str = computed_description();
    /// Required associated value with no trait default.
    const REQUIRED: &'static str;
}

/// Inherit trait defaults and provide a computed required value.
struct DefaultDescription;

impl DescriptionValue for DefaultDescription {
    /// Keep the required value behind a const computation.
    const REQUIRED: &'static str = computed_description();
}

/// Override the empty trait default with a nonempty description.
struct NamedDescription;

impl DescriptionValue for NamedDescription {
    /// Replace the empty trait default with reviewer context.
    const OVERRIDDEN: &'static str = "API response";
    /// Provide a known nonempty required value.
    const REQUIRED: &'static str = "API response";
}

/// Keep trait defaults, overrides, and computed values unknown to the lint.
fn trait_associated_description_controls(
    _default_description: DefaultDescription,
    _named_description: NamedDescription,
) {
    let mut settings = insta::Settings::clone_current();
    settings.set_description(<DefaultDescription as DescriptionValue>::DEFAULT_EMPTY);
    settings.set_description(<NamedDescription as DescriptionValue>::OVERRIDDEN);
    settings.set_description(<DefaultDescription as DescriptionValue>::COMPUTED);
    settings.set_description(<DefaultDescription as DescriptionValue>::REQUIRED);
}
