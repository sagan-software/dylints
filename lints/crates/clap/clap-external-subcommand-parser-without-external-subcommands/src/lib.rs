#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks inactive external-subcommand value parsers.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use clap as _;
use rustc_lint::LintContext as _;

clap_support::declare_builder_requirement_lint! {
    CLAP_EXTERNAL_SUBCOMMAND_PARSER_WITHOUT_EXTERNAL_SUBCOMMANDS,
    ClapExternalSubcommandParserWithoutExternalSubcommands,
    Command,
    "external_subcommand_value_parser",
    false,
    "allow_external_subcommands",
    true,
    "a clap external-subcommand parser is inactive",
    "this parser is ignored unless external subcommands are enabled",
    "add `.allow_external_subcommands(true)`"
}
