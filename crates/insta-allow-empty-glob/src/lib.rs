#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks unconditional disabling of Insta's empty-glob safeguard.
//!
//! This Dylint library resolves Insta settings, reports unconditional empty-glob
//! allowance, and recommends preserving the safeguard during snapshot review.
//!
//! The README defines the supported setting and replacement. UI fixtures cover
//! triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_settings_method_lint! {
    INSTA_ALLOW_EMPTY_GLOB,
    InstaAllowEmptyGlob,
    "set_allow_empty_glob",
    TrueArgument,
    "Insta empty-glob failures are disabled",
    "allowing an empty glob can hide a misspelled or missing fixture path",
    "keep the default unless an empty fixture set is intentionally valid"
}
