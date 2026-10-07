#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks negative clap values without an arity.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Clap argument builders, reports negative-value
//! settings without an arity, and recommends declaring the expected arguments.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_ALLOW_NEGATIVE_NUMBERS_WITHOUT_NUM_ARGS,
    ClapAllowNegativeNumbersWithoutNumArgs,
    clap_support::BuilderRequirement {
        builder: clap_support::BuilderType::Arg,
        trigger: "allow_negative_numbers",
        is_trigger_true_required: true,
        required: "num_args",
        is_requirement_true_required: false,
        satisfying_actions: &["Set", "Append"],
    },
    "a clap argument accepts negative numbers without an arity",
    "`allow_negative_numbers(true)` is ambiguous without `num_args`",
    "add an explicit `.num_args(...)`"
}
