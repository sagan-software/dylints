#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks mutable queries that directly manipulate Bevy Children.
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
    BEVY_CHILDREN_MUTATION_QUERY,
    BevyChildrenMutationQuery,
    bevy_support::children_mutation_query_parameters,
    "checks mutable queries that directly manipulate Bevy Children",
    "`Children` should not be manipulated directly",
    "modify `ChildOf` on source entities or use relationship commands"
}
