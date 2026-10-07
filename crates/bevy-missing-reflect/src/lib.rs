#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks Bevy domain types that omit reflection.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;

use dylint_linting as _;
use rustc_lint::LintContext as _;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_MISSING_REFLECT,
    Warn,
    "a Bevy domain type does not implement Reflect",
    BevyMissingReflect
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyMissingReflect {
    /// Check local Bevy domain types for `Reflect`.
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        // Types that exist only in a test harness never reach scenes or inspectors.
        if bevy_support::is_test_harness(cx) {
            return;
        }
        for target in bevy_support::local_bevy_types_missing_reflect(cx) {
            cx.emit_span_lint(
                BEVY_MISSING_REFLECT,
                cx.tcx.def_span(target),
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this Bevy domain type does not implement `Reflect`")
                        .help("derive `Reflect`");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}

/// Compile the test-harness fixture as rustc does for `cargo test`.
#[test]
fn ui_test_harness() {
    dylint_testing::ui::Test::example(env!("CARGO_PKG_NAME"), "test_harness")
        .rustc_flags(["--test"])
        .run();
}
