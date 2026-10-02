#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks for panicking tracked Bevy World resource access.
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

bevy_support::declare_world_method_lint! {
    BEVY_WORLD_RESOURCE_REF,
    BevyWorldResourceRef,
    Warn,
    "resource_ref",
    "get_resource_ref",
    "checks for panicking tracked Bevy World resource access",
    "`World::resource_ref` panics when the resource is absent"
}
