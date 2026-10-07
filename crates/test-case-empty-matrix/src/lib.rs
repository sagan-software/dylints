#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for test-case matrices that generate no tests.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_EMPTY_MATRIX,
    TestCaseEmptyMatrix,
    EmptyMatrix,
    "a test-case matrix generates no tests",
    "this test matrix generates no test cases",
    "populate every input set so the Cartesian product is nonempty"
}
