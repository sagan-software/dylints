#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks inactive external-subcommand value parsers.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
use clap as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_EXTERNAL_SUBCOMMAND_PARSER_WITHOUT_EXTERNAL_SUBCOMMANDS,
    ClapExternalSubcommandParserWithoutExternalSubcommands,
    clap_support::BuilderRequirement {
        builder: clap_support::BuilderType::Command,
        trigger: "external_subcommand_value_parser",
        is_trigger_true_required: false,
        required: "allow_external_subcommands",
        is_requirement_true_required: true,
        satisfying_actions: &[],
    },
    "a clap external-subcommand parser is inactive",
    "this parser is ignored unless external subcommands are enabled",
    "add `.allow_external_subcommands(true)`"
}
