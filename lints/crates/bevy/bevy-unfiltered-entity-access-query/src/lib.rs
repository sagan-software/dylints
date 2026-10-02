#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks broad entity queries followed by fixed typed access.
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
use bevy_ecs as _;

bevy_support::declare_function_parameter_lint! {
    BEVY_UNFILTERED_ENTITY_ACCESS_QUERY,
    BevyUnfilteredEntityAccessQuery,
    bevy_support::unfiltered_entity_access_query_parameters,
    "a broad entity query is followed by fixed typed component access",
    "this whole-entity query performs fixed typed component access",
    "request the fixed component set directly in the query"
}
