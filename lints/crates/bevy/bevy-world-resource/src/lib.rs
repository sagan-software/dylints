#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks panicking Bevy `World::resource` access.
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
    BEVY_WORLD_RESOURCE,
    BevyWorldResource,
    Warn,
    "resource",
    "get_resource",
    "checks for panicking Bevy World resource access",
    "`World::resource` panics when the resource is absent"
}
