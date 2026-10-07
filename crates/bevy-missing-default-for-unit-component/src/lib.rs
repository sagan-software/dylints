#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks unit Bevy components missing Default.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;

use dylint_linting as _;

#[cfg(test)]
use bevy as _;

bevy_support::declare_missing_unit_component_trait_lint! {
    BEVY_MISSING_DEFAULT_FOR_UNIT_COMPONENT,
    BevyMissingDefaultForUnitComponent,
    Default,
    "checks unit Bevy components missing Default",
    "this unit component does not implement `Default`",
    "derive or implement `Default`"
}
