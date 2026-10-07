#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks hyphenated clap values without an arity.
//!
//! This Dylint library resolves Clap argument builders, reports hyphenated
//! values without an arity, and recommends an explicit argument range.
//!
//! The README defines supported call shapes and the replacement. UI fixtures
//! cover triggering and non-triggering forms so unrelated code remains unchanged.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_ALLOW_HYPHEN_VALUES_WITHOUT_NUM_ARGS,
    ClapAllowHyphenValuesWithoutNumArgs,
    clap_support::BuilderRequirement {
        builder: clap_support::BuilderType::Arg,
        trigger: "allow_hyphen_values",
        is_trigger_true_required: true,
        required: "num_args",
        is_requirement_true_required: false,
        satisfying_actions: &["Set", "Append"],
    },
    "a clap argument accepts hyphen values without an arity",
    "`allow_hyphen_values(true)` is ambiguous without `num_args`",
    "add an explicit `.num_args(...)`"
}
