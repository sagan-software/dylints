#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks duplicate direct versions of the Bevy facade crate.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;
use rustc_lint::LintContext as _;

#[cfg(test)]
use {bevy_018 as _, bevy_019 as _};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_DUPLICATE_DEPENDENCIES,
    Warn,
    "multiple versions of the Bevy facade are loaded directly",
    BevyDuplicateDependencies
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyDuplicateDependencies {
    /// Count loaded direct Bevy facade crates after crate analysis.
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        if bevy_support::loaded_bevy_facades(cx) < 2 {
            return;
        }
        cx.emit_span_lint(
            BEVY_DUPLICATE_DEPENDENCIES,
            cx.tcx.hir_span(rustc_hir::CRATE_HIR_ID),
            rustc_errors::DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("multiple versions of the `bevy` facade are loaded")
                    .help("align direct Bevy dependencies to one version");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
