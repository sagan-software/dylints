#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks trailing variable arguments without an arity.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Clap argument builders, reports trailing
//! variable arguments without an arity, and recommends declaring their range.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_TRAILING_VAR_ARG_WITHOUT_NUM_ARGS,
    ClapTrailingVarArgWithoutNumArgs,
    clap_support::BuilderRequirement {
        builder: clap_support::BuilderType::Arg,
        trigger: "trailing_var_arg",
        is_trigger_true_required: true,
        required: "num_args",
        is_requirement_true_required: false,
        satisfying_actions: &["Append"],
    },
    "a clap trailing variable argument has no arity",
    "`trailing_var_arg(true)` requires `num_args`",
    "add `.num_args(...)` to this argument"
}
