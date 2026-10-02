#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks change filters on large Bevy components.
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
    BEVY_LARGE_COMPONENT_CHANGE_FILTER,
    BevyLargeComponentChangeFilter,
    bevy_support::large_component_change_filter_parameters,
    "a change filter tracks an unusually large Bevy component",
    "this query change-tracks a large component",
    "split independently changing state and filter on the specific component"
}
