#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks the legacy test-case inconclusive modifier.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_INCONCLUSIVE_MODIFIER,
    TestCaseInconclusiveModifier,
    InconclusiveModifier,
    "a test-case attribute uses the inconclusive modifier",
    "`inconclusive` generates an ignored Rust test",
    "use `ignore[\"reason\"]` to state that behavior directly"
}
