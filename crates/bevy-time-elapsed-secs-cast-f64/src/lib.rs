#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks lossy widening of Bevy elapsed seconds.
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
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_TIME_ELAPSED_SECS_CAST_F64,
    Warn,
    "checks lossy widening of Bevy elapsed seconds",
    BevyTimeElapsedSecsCastF64
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyTimeElapsedSecsCastF64 {
    /// Check `as`, `From`, and `Into` widenings of `Time::elapsed_secs()`.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        let Some(widening) = bevy_support::elapsed_secs_widening(cx, expr) else {
            return;
        };
        cx.emit_span_lint(
            BEVY_TIME_ELAPSED_SECS_CAST_F64,
            widening.method_span,
            rustc_errors::DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic.primary_message(
                    "widening `elapsed_secs()` to `f64` cannot recover its lost precision",
                );
                // Rename the call and drop the conversion syntax when the source is rewritable.
                if widening.removals.is_empty() {
                    let _helped = diagnostic.help("call `elapsed_secs_f64()` instead");
                    return;
                }
                let replacements =
                    core::iter::once((widening.method_span, String::from("elapsed_secs_f64")))
                        .chain(widening.removals.iter().map(|span| (*span, String::new())))
                        .collect();
                let _suggested = diagnostic.multipart_suggestion(
                    "call `elapsed_secs_f64()` instead",
                    replacements,
                    rustc_errors::Applicability::MachineApplicable,
                );
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
