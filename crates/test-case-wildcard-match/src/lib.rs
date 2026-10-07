#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks vacuous wildcard test-case match assertions.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_WILDCARD_MATCH,
    TestCaseWildcardMatch,
    WildcardMatch,
    "a test-case match assertion uses only a wildcard",
    "the wildcard pattern accepts every returned value",
    "match the expected variant or value"
}
