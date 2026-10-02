#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks unit values passed inside Bevy bundles.
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
use bevy_ecs as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_UNIT_IN_BUNDLE,
    Warn,
    "checks unit values passed inside Bevy bundles",
    BevyUnitInBundle
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyUnitInBundle {
    /// Check bundle arguments of Bevy spawn and insert methods for unit values.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        for value in bevy_support::unit_bundle_values(cx, expr) {
            cx.emit_span_lint(
                BEVY_UNIT_IN_BUNDLE,
                value.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("unit values are skipped when Bevy inserts this bundle");
                    // Offer the removal only for literal units with an exact rewrite.
                    if value.replacements.is_empty() {
                        let _helped = diagnostic
                            .help("remove the unit value or use `spawn_empty` for an empty spawn");
                    } else {
                        let _suggested = diagnostic.multipart_suggestion(
                            value.suggestion,
                            value.replacements.clone(),
                            rustc_errors::Applicability::MachineApplicable,
                        );
                    }
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
