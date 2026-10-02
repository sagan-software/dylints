#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks camera mutation scheduled in Bevy's fixed update.
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
use {bevy_app as _, bevy_camera as _, bevy_ecs as _, bevy_transform as _};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_CAMERA_MODIFICATION_IN_FIXED_UPDATE,
    Warn,
    "a camera-mutating system is scheduled in FixedUpdate",
    BevyCameraModificationInFixedUpdate
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyCameraModificationInFixedUpdate {
    /// Check direct systems registered in `FixedUpdate`.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        for span in bevy_support::camera_fixed_update_system_spans(cx, expr) {
            cx.emit_span_lint(
                BEVY_CAMERA_MODIFICATION_IN_FIXED_UPDATE,
                span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "this system mutates camera-filtered query data in `FixedUpdate`",
                        )
                        .help("schedule camera mutation in a frame-rate update schedule");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
