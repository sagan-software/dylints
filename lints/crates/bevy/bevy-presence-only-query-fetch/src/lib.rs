#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks component fetches used only for query presence.
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

bevy_support::declare_function_parameter_lint! {
    BEVY_PRESENCE_ONLY_QUERY_FETCH,
    BevyPresenceOnlyQueryFetch,
    bevy_support::presence_only_query_parameters,
    "a Bevy query fetches a component used only for presence or count",
    "this query fetches component values but only checks presence or count",
    "use an Entity query with a With<T> filter"
}
