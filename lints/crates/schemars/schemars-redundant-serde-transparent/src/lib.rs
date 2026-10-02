#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks redundant Schemars transparent attributes.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

use rustc_lint::LintContext as _;
#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

schemars_support::declare_redundant_serde_attribute_lint! {
    SCHEMARS_REDUNDANT_SERDE_TRANSPARENT,
    SchemarsRedundantSerdeTransparent,
    ["transparent"],
    "a Schemars transparent attribute duplicates Serde"
}
