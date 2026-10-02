#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks disabled Insta module prefixes.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_settings_method_lint! {
    INSTA_NO_MODULE_PREFIX,
    InstaNoModulePrefix,
    "set_prepend_module_to_snapshot",
    FalseArgument,
    "Insta module prefixes are disabled",
    "disabling module prefixes can collide snapshot names",
    "keep the default module prefix"
}
