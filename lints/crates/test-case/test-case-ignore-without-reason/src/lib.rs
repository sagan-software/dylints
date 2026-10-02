#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks ignore-without-reason test-case suites.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves test-case ignore attributes, reports ignored
//! cases without an explanation, and recommends adding a reviewable reason.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_IGNORE_WITHOUT_REASON,
    TestCaseIgnoreWithoutReason,
    IgnoreWithoutReason,
    "a test-case ignore modifier has no reason",
    "this ignored case does not explain why",
    "add an ignore reason"
}
