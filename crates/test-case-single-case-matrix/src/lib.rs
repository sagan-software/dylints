#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for test-case matrices that generate one test.
//!
//! This Dylint library resolves test-case matrices, reports one-case matrix
//! attributes, and recommends a direct test case or a matrix with meaningful
//! variation. A scalar test communicates the intended number of generated
//! cases without requiring matrix expansion.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_SINGLE_CASE_MATRIX,
    TestCaseSingleCaseMatrix,
    SingleCaseMatrix,
    "a test-case matrix generates only one test",
    "this test matrix generates only one test case",
    "replace the matrix with one `#[test_case(...)]` attribute"
}
