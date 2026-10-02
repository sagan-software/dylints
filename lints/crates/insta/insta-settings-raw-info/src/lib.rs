#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Insta raw snapshot info.
//!
//! This Dylint library resolves Insta settings, reports raw snapshot metadata
//! that obscures review, and recommends the structured setting API.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_settings_method_lint! {
    INSTA_SETTINGS_RAW_INFO,
    InstaSettingsRawInfo,
    "set_raw_info",
    Any,
    "Insta Settings uses raw snapshot info",
    "raw snapshot info bypasses redactions",
    "use `set_info` when redactions should apply"
}
