#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks redundant Schemars rename attributes.
//!
//! This Dylint library resolves Schemars and Serde rename attributes, reports
//! duplicate names, and recommends retaining one authoritative declaration.
//!
//! The README defines the supported attribute shapes and replacement. UI
//! fixtures cover triggering and non-triggering forms without changing callers.

extern crate rustc_ast;
extern crate rustc_errors;

use rustc_lint::LintContext as _;
#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

schemars_support::declare_redundant_serde_attribute_lint! {
    SCHEMARS_REDUNDANT_SERDE_RENAME,
    SchemarsRedundantSerdeRename,
    "rename",
    "a Schemars rename attribute duplicates Serde"
}
