#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks use of Insta's deprecated display assertion macro.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;
use rustc_lint::LintContext as _;

insta_support::declare_discouraged_macro_replacement_lint! {
    INSTA_DEPRECATED_ASSERT_DISPLAY,
    InstaDeprecatedAssertDisplay,
    "assert_display_snapshot",
    "assert_snapshot",
    "Insta's deprecated assert_display_snapshot macro is used",
    "`assert_display_snapshot!` is deprecated",
    "use `assert_snapshot!`, which accepts the same `Display` values"
}
