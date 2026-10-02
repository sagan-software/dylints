#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks descriptions that rely on removed `inconclusive` semantics.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_LEGACY_INCONCLUSIVE_DESCRIPTION,
    TestCaseLegacyInconclusiveDescription,
    LegacyInconclusiveDescription,
    "a test-case description uses the removed inconclusive convention",
    "an `inconclusive` description no longer ignores this generated test",
    "use `ignore[\"reason\"]` as an output modifier when the case must be skipped"
}
