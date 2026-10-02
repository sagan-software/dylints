#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty test-case ignore reasons.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves test-case ignore attributes, reports empty
//! reasons, and recommends describing why the case is intentionally ignored.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_EMPTY_IGNORE_REASON,
    TestCaseEmptyIgnoreReason,
    EmptyIgnoreReason,
    "a test-case ignore modifier has an empty reason",
    "this ignored case has an empty reason",
    "describe why the case is ignored"
}
