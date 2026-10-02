#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks large-suite test-case suites.
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
    TEST_CASE_LARGE_SUITE,
    TestCaseLargeSuite,
    LargeSuite,
    "a test-case macro generates a large suite",
    "this macro generates at least 64 tests",
    "split the matrix or reduce its Cartesian product"
}
