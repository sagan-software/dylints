#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Insta scope guards in async code.
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
    INSTA_BIND_TO_SCOPE_IN_ASYNC,
    InstaBindToScopeInAsync,
    "bind_to_scope",
    InAsyncBody,
    "Insta Settings is bound to a thread-local scope in async code",
    "this thread-local guard can cross async scheduling points",
    "use `Settings::bind_async`"
}
