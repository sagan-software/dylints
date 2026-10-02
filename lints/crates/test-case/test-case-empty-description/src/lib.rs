#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty test-case descriptions.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_EMPTY_DESCRIPTION,
    TestCaseEmptyDescription,
    EmptyDescription,
    "a test-case attribute has an empty description",
    "this case has an empty explicit description",
    "describe the behavior covered by this case"
}
