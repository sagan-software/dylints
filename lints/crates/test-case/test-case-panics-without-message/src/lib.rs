#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks panics-without-message test-case suites.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves test-case panic attributes, reports cases
//! without an expected message, and recommends adding the asserted message.

extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

test_case_support::declare_suite_lint! {
    TEST_CASE_PANICS_WITHOUT_MESSAGE,
    TestCasePanicsWithoutMessage,
    PanicsWithoutMessage,
    "a test-case panics modifier has no expected message",
    "this case accepts any panic",
    "add the expected panic message"
}
