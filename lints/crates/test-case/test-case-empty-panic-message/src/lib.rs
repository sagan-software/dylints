#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks empty test-case panic messages.
//!
//! This Dylint library resolves test-case attributes, reports empty expected
//! panic messages, and recommends a stable message substring for the case.
//!
//! The README defines the supported attribute shapes and replacement. UI
//! fixtures cover triggering and non-triggering forms for safe adoption.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_EMPTY_PANIC_MESSAGE,
    TestCaseEmptyPanicMessage,
    EmptyPanicMessage,
    "a test-case panics modifier has an empty expected message",
    "this case accepts every panic message",
    "provide a stable expected panic substring"
}
