#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks unit values passed inside Bevy bundles.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;

#[cfg(test)]
use bevy_ecs as _;

bevy_support::declare_expression_spans_lint! {
    BEVY_UNIT_IN_BUNDLE,
    BevyUnitInBundle,
    Warn,
    bevy_support::unit_bundle_spans,
    "checks unit values passed inside Bevy bundles",
    "unit values are skipped when Bevy inserts this bundle",
    "remove the unit value or use `spawn_empty` for an empty spawn"
}
