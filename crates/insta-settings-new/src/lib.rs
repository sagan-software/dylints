#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `Settings::new`.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Insta settings constructors, reports creation
//! that discards inherited settings, and recommends cloning current settings.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SETTINGS_NEW,
    Warn,
    "Insta Settings::new discards current settings",
    InstaSettingsNew
}

impl<'tcx> LateLintPass<'tcx> for InstaSettingsNew {
    /// Check calls that build `insta::Settings` from Insta's defaults.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(call) = insta_support::settings_defaults_call(cx, expr) else {
            return;
        };
        cx.emit_span_lint(
            INSTA_SETTINGS_NEW,
            expr.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this `Settings` value discards inherited configuration");
                // Renaming the constructor keeps the type path and the empty argument list.
                if let Some(name_span) = call.name_span {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        name_span,
                        "use `Settings::clone_current`",
                        "clone_current",
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help = diagnostic.help("use `Settings::clone_current()`");
                }
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
