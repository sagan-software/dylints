#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks entrypoints that discard Bevy's requested exit status.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use dylint_linting as _;
use rustc_lint::LintContext as _;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_MAIN_RETURN_WITHOUT_APP_EXIT,
    Warn,
    "a unit-returning main discards Bevy AppExit",
    BevyMainReturnWithoutAppExit
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyMainReturnWithoutAppExit {
    /// Check a unit-returning entrypoint for discarded `App::run` results.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        _: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: rustc_span::Span,
        local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        for span in bevy_support::discarded_app_run_spans(cx, declaration, body, local_def_id) {
            cx.emit_span_lint(
                BEVY_MAIN_RETURN_WITHOUT_APP_EXIT,
                span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this entrypoint discards Bevy's requested exit status")
                        .help("return `AppExit` from `main` and tail-return `App::run`");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
