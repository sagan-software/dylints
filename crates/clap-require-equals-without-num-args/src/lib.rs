#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks equals-delimited clap values without an arity.
//!
//! This Dylint library resolves Clap argument builders, reports equals-delimited
//! values without an arity, and recommends declaring their accepted range.
//!
//! The README defines the supported call shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_REQUIRE_EQUALS_WITHOUT_NUM_ARGS,
    ClapRequireEqualsWithoutNumArgs,
    clap_support::BuilderRequirement {
        builder: clap_support::BuilderType::Arg,
        trigger: "require_equals",
        is_trigger_true_required: true,
        required: "num_args",
        is_requirement_true_required: false,
        satisfying_actions: &["Set", "Append"],
    },
    "a clap argument requires equals without an arity",
    "`require_equals(true)` has no configured value arity",
    "add an explicit `.num_args(...)`"
}
