#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks singleton contains-in-order expectations in test-case assertions.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_SINGLETON_CONTAINS_IN_ORDER,
    TestCaseSingletonContainsInOrder,
    SingletonContainsInOrder,
    "a test-case contains_in_order matcher has one element",
    "ordering is not observable for a one-element expected sequence",
    "use the `contains` matcher for the expected element"
}
