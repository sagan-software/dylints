#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks test matrices that ignore every generated case.
//!
//! This Dylint library resolves test-case matrices, reports suites that ignore
//! every generated case, and recommends retaining at least one exercised case.
//!
//! The README defines the supported matrix shapes and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_IGNORED_MATRIX,
    TestCaseIgnoredMatrix,
    IgnoredMatrix,
    "a test matrix ignores every generated case",
    "the matrix-wide ignore modifier disables every combination",
    "remove the modifier or express exceptional ignored inputs as separate test cases"
}
