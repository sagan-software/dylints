#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks unnamed-test-case test-case suites.
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
    TEST_CASE_UNNAMED_TEST_CASE,
    TestCaseUnnamedTestCase,
    UnnamedTestCase,
    "test_case cases have generated names",
    "this case has no explicit display name",
    "add a descriptive case comment after `;`"
}
