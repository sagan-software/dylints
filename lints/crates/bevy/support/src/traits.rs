//! Semantic analysis for Bevy marker traits and naming conventions.

use super::helpers::{
    local_item_is_unit_struct, local_standard_trait_targets, local_trait_targets,
};
use super::{LateContext, LocalDefId, MarkerTrait, Span};

/// Return local component-like types that lack `Reflect`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::local_bevy_types_missing_reflect(cx);
/// };
/// ```
pub fn local_bevy_types_missing_reflect(cx: &LateContext<'_>) -> Vec<LocalDefId> {
    // Build the exclusion set once before scanning each Bevy marker trait.
    let reflected = local_trait_targets(cx, "bevy_reflect", "Reflect");
    let mut targets = Vec::new();

    for (crate_name, trait_name) in [
        ("bevy_ecs", "Component"),
        ("bevy_ecs", "Resource"),
        ("bevy_ecs", "Message"),
        ("bevy_ecs", "Event"),
    ] {
        // Deduplicate types that implement more than one Bevy marker trait.
        for target in local_trait_targets(cx, crate_name, trait_name) {
            if reflected.contains(&target) || targets.contains(&target) {
                continue;
            }
            targets.push(target);
        }
    }

    targets
}

/// Return local unit components missing one standard trait.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, marker_trait| {
///     let _ = bevy_support::local_unit_components_missing_trait(cx, marker_trait);
/// };
/// ```
pub fn local_unit_components_missing_trait(
    cx: &LateContext<'_>,
    marker_trait: MarkerTrait,
) -> impl Iterator<Item = LocalDefId> {
    let implemented = local_standard_trait_targets(cx, marker_trait);

    local_trait_targets(cx, "bevy_ecs", "Component")
        .into_iter()
        .filter(|target| local_item_is_unit_struct(cx, *target))
        .filter(move |target| !implemented.contains(target))
}

/// Return an attribute insertion that derives a trait on a unit component.
///
/// The result is `None` when the derive could fail to compile: when the item
/// comes from a macro expansion, or when `Copy` is missing its `Clone` supertrait.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, target, marker_trait| {
///     let _ = bevy_support::missing_trait_derive(cx, target, marker_trait);
/// };
/// ```
pub fn missing_trait_derive(
    cx: &LateContext<'_>,
    target: LocalDefId,
    marker_trait: MarkerTrait,
) -> Option<(Span, String)> {
    let span = cx.tcx.def_span(target);
    if span.from_expansion() {
        return None;
    }
    // `Copy` requires `Clone`, so its derive compiles only next to an existing `Clone`.
    if marker_trait == MarkerTrait::Copy
        && !local_standard_trait_targets(cx, MarkerTrait::Clone).contains(&target)
    {
        return None;
    }
    Some((
        span.shrink_to_lo(),
        format!("#[derive({})]\n", marker_trait.name()),
    ))
}

/// Return local Bevy trait implementations whose names violate conventions.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::unconventional_bevy_type_names(cx);
/// };
/// ```
pub fn unconventional_bevy_type_names(
    cx: &LateContext<'_>,
) -> Vec<(LocalDefId, &'static str, &'static str)> {
    // Accumulate one violation per local type across the supported Bevy traits.
    let mut violations = Vec::new();

    for (trait_name, suffix) in [("Plugin", "Plugin"), ("SystemSet", "Systems")] {
        // Search both crates that expose the naming-constrained traits.
        for target in local_trait_targets(cx, "bevy_app", trait_name)
            .into_iter()
            .chain(local_trait_targets(cx, "bevy_ecs", trait_name))
        {
            let name = cx.tcx.item_name(target.to_def_id());
            let has_expected_suffix = name.as_str().ends_with(suffix);
            if has_expected_suffix {
                continue;
            }
            // A type implementing both resolved traits still needs one diagnostic.
            let is_already_reported = violations
                .iter()
                .any(|(existing, _, _)| *existing == target);
            if !is_already_reported {
                violations.push((target, trait_name, suffix));
            }
        }
    }

    violations
}

/// Count direct dependencies whose original crate name is `bevy`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = bevy_support::direct_bevy_facades(cx);
/// };
/// ```
pub fn direct_bevy_facades(cx: &LateContext<'_>) -> usize {
    cx.tcx
        .crates(())
        .iter()
        .filter(|&&crate_num| {
            cx.tcx.crate_name(crate_num).as_str() == "bevy"
                && cx
                    .tcx
                    .extern_crate(crate_num)
                    .is_some_and(rustc_session::cstore::ExternCrate::is_direct)
        })
        .count()
}
