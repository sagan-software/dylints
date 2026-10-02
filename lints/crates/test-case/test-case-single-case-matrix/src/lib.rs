#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for test-case matrices that generate one test.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves test-case matrices, reports one-case suites,
//! and recommends a direct test or a matrix with meaningful variation.

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
    pub TEST_CASE_SINGLE_CASE_MATRIX,
    Warn,
    "a test-case matrix generates only one test",
    TestCaseSingleCaseMatrix
}

impl<'tcx> LateLintPass<'tcx> for TestCaseSingleCaseMatrix {
    /// Check each resolved test-case-generated suite module.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        // Report only matrix expansions that produced exactly one generated test.
        let Some(suite) = generated_test_suite(cx, item) else {
            return;
        };
        if suite.macro_kind != TestCaseMacro::TestMatrix || suite.tests.len() != 1 {
            return;
        }

        cx.emit_span_lint(
            TEST_CASE_SINGLE_CASE_MATRIX,
            suite.call_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this test matrix generates only one test case")
                    .help("replace the matrix with one `#[test_case(...)]` attribute");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
