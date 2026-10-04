#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks duplicate Bevy plugin additions that may panic in one application.
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
use bevy_app as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_DUPLICATE_PLUGIN_ADDITION,
    Warn,
    "checks duplicate unique Bevy plugin additions",
    BevyDuplicatePluginAddition
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyDuplicatePluginAddition {
    /// Check direct additions to one proven local `App` within a block.
    fn check_block(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        block: &'tcx rustc_hir::Block<'tcx>,
    ) {
        for duplicate in bevy_support::duplicate_plugin_additions_in_block(cx, block) {
            cx.emit_span_lint(
                BEVY_DUPLICATE_PLUGIN_ADDITION,
                duplicate.method_span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic =
                        diagnostic.primary_message("this app already has this unique plugin");
                    let _helped = diagnostic.help("remove this duplicate `add_plugins` call");
                }),
            );
        }
    }

    /// Check chained `add_plugins` calls that repeat one unique plugin.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        // Keep tuple elements intact because their expressions may have side effects.
        if let Some(method_span) = bevy_support::tuple_duplicate_plugin_addition(cx, expr) {
            cx.emit_span_lint(
                BEVY_DUPLICATE_PLUGIN_ADDITION,
                method_span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic.primary_message(
                        "this tuple adds the same unique plugin type more than once",
                    );
                    let _helped =
                        diagnostic.help("remove one of the duplicate plugins from the tuple");
                }),
            );
            return;
        }
        let Some(duplicate) = bevy_support::duplicate_plugin_addition(cx, expr) else {
            return;
        };
        cx.emit_span_lint(
            BEVY_DUPLICATE_PLUGIN_ADDITION,
            duplicate.method_span,
            rustc_errors::DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this unique plugin is added twice in the same chain");
                // Delete the repeated call only when its argument cannot run code.
                if let Some(removal) = duplicate.removal {
                    let _suggested = diagnostic.span_suggestion_verbose(
                        removal,
                        "remove the duplicate `add_plugins` call",
                        "",
                        rustc_errors::Applicability::MachineApplicable,
                    );
                } else {
                    let _helped = diagnostic.help("remove the duplicate `add_plugins` call");
                }
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
