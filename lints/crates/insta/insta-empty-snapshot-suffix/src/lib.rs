#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty Insta snapshot snapshot suffixs.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_empty_settings_lint! {
    INSTA_EMPTY_SNAPSHOT_SUFFIX,
    InstaEmptySnapshotSuffix,
    "set_snapshot_suffix",
    "an Insta snapshot snapshot suffix is empty",
    "provide reviewer context or omit the setting"
}
