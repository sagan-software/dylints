#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks constant boolean guards in test-case match assertions.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_CONSTANT_MATCH_GUARD,
    TestCaseConstantMatchGuard,
    ConstantMatchGuard,
    "a test-case match assertion has a constant guard",
    "this match guard is always true or always false",
    "remove a true guard or replace a false guard with the intended condition"
}
