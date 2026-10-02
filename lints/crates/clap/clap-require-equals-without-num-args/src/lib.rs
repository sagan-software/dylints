#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks equals-delimited clap values without an arity.
//!
//! This Dylint library resolves Clap argument builders, reports equals-delimited
//! values without an arity, and recommends declaring their accepted range.
//!
//! The README defines the supported call shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use clap as _;
use rustc_lint::LintContext as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_REQUIRE_EQUALS_WITHOUT_NUM_ARGS,
    ClapRequireEqualsWithoutNumArgs,
    Arg,
    "require_equals",
    true,
    "num_args",
    false,
    "a clap argument requires equals without an arity",
    "`require_equals(true)` has no configured value arity",
    "add an explicit `.num_args(...)`"
}
