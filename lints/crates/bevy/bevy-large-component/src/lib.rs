#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks large Bevy components.
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
use bevy_ecs as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_LARGE_COMPONENT,
    Warn,
    "a Bevy component combines many fields in a large layout",
    BevyLargeComponent
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyLargeComponent {
    /// Check local component definitions after trait resolution.
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        for component in bevy_support::local_large_components(cx) {
            cx.emit_span_lint(
                BEVY_LARGE_COMPONENT,
                cx.tcx.def_span(component),
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "this component has at least eight fields and exceeds 64 bytes",
                        )
                        .help("split independently accessed state into cohesive components");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
