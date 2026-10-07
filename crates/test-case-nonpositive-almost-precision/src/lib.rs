#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks nonpositive literal precision in test-case almost matchers.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_NONPOSITIVE_ALMOST_PRECISION,
    TestCaseNonpositiveAlmostPrecision,
    NonpositiveAlmostPrecision,
    "a test-case almost matcher has nonpositive precision",
    "an absolute difference cannot be less than this precision",
    "use a positive tolerance that reflects the expected numerical error"
}
