#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks redundant Schemars `deny_unknown_fields` attributes.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Schemars and Serde attributes, reports
//! duplicate unknown-field policies, and recommends one authoritative marker.

extern crate rustc_ast;
extern crate rustc_errors;

use rustc_lint::LintContext as _;
#[cfg(test)]
use schemars as _;
#[cfg(test)]
use serde as _;

schemars_support::declare_redundant_serde_attribute_lint! {
    SCHEMARS_REDUNDANT_SERDE_DENY_UNKNOWN_FIELDS,
    SchemarsRedundantSerdeDenyUnknownFields,
    ["deny_unknown_fields"],
    "a Schemars deny_unknown_fields attribute duplicates Serde"
}
