#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks whitespace-only test-case panic messages.
//!
//! This Dylint library resolves test-case panic attributes, reports messages
//! containing only whitespace, and recommends a stable expected message.
//!
//! The README defines the supported attribute shapes and replacement. UI
//! fixtures cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

use rustc_lint::LintContext as _;
#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_WHITESPACE_PANIC_MESSAGE,
    TestCaseWhitespacePanicMessage,
    WhitespacePanicMessage,
    "a test-case panic expectation contains only whitespace",
    "this expected substring does not identify the intended panic",
    "match a stable, nonblank part of the intended panic message"
}
