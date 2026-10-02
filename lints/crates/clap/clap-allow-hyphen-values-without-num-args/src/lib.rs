#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks hyphenated clap values without an arity.
//!
//! This Dylint library resolves Clap argument builders, reports hyphenated
//! values without an arity, and recommends an explicit argument range.
//!
//! The README defines supported call shapes and the replacement. UI fixtures
//! cover triggering and non-triggering forms so unrelated code remains unchanged.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use clap as _;
use rustc_lint::LintContext as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_ALLOW_HYPHEN_VALUES_WITHOUT_NUM_ARGS,
    ClapAllowHyphenValuesWithoutNumArgs,
    Arg,
    "allow_hyphen_values",
    true,
    "num_args",
    false,
    "a clap argument accepts hyphen values without an arity",
    "`allow_hyphen_values(true)` is ambiguous without `num_args`",
    "add an explicit `.num_args(...)`"
}
