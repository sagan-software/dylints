#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks whitespace-only test-case ignore reasons.
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
    TEST_CASE_WHITESPACE_IGNORE_REASON,
    TestCaseWhitespaceIgnoreReason,
    WhitespaceIgnoreReason,
    "a test-case ignore reason contains only whitespace",
    "this ignore reason does not explain why the case is disabled",
    "record the blocker or tracked issue"
}
