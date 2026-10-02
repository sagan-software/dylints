#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks wide Bevy query access.
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

bevy_support::declare_function_parameter_type_lint! {
    BEVY_WIDE_QUERY_ACCESS,
    BevyWideQueryAccess,
    bevy_support::wide_query_parameters,
    "a Bevy query requests an unusually wide component access set",
    "this query requests a wide component access set",
    "split the system by behavior and request only the components it uses"
}
