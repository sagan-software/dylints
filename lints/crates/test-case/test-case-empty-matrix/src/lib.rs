#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for test-case matrices that generate no tests.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
extern crate test_case as _;

use rustc_errors::DiagDecorator;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use test_case_support::{TestCaseMacro, generated_test_suite};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TEST_CASE_EMPTY_MATRIX,
    Warn,
    "a test-case matrix generates no tests",
    TestCaseEmptyMatrix
}

impl<'tcx> LateLintPass<'tcx> for TestCaseEmptyMatrix {
    /// Check each resolved test-case-generated suite module.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Report only matrix expansions that produced no generated tests.
        let Some(suite) = generated_test_suite(cx, item) else {
            return;
        };
        if suite.macro_kind != TestCaseMacro::TestMatrix || !suite.tests.is_empty() {
            return;
        }

        cx.emit_span_lint(
            TEST_CASE_EMPTY_MATRIX,
            suite.call_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this test matrix generates no test cases")
                    .help("populate every input set so the Cartesian product is nonempty");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
