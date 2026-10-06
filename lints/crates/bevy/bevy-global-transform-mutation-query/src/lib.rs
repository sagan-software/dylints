#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks mutable queries of Bevy-managed `GlobalTransform`.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use dylint_linting as _;

#[cfg(test)]
use bevy as _;
#[cfg(test)]
use bevy as _;

bevy_support::declare_mutable_query_component_lint! {
    BEVY_GLOBAL_TRANSFORM_MUTATION_QUERY,
    BevyGlobalTransformMutationQuery,
    "bevy_transform",
    "GlobalTransform",
    "checks mutable queries of Bevy-managed GlobalTransform",
    "`GlobalTransform` is maintained by Bevy and cannot be mutated directly",
    "query and mutate `Transform` instead"
}
