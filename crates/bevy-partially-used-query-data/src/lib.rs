#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks broad custom `QueryData` use.
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
    BEVY_PARTIALLY_USED_QUERY_DATA,
    BevyPartiallyUsedQueryData,
    bevy_support::partially_used_query_data_parameters,
    "a broad custom QueryData type is only partly used",
    "this custom query uses at most half of its fields",
    "define a smaller query type for this behavior"
}
