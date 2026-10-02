#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks async test-case suites without an async test harness.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;
#[cfg(test)]
extern crate tokio as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_ASYNC_WITHOUT_TEST_HARNESS,
    TestCaseAsyncWithoutTestHarness,
    AsyncWithoutTestHarness,
    "an async test-case suite has no async test harness",
    "these generated async functions are not registered as tests",
    "place an async runtime test attribute after the test-case attributes"
}
