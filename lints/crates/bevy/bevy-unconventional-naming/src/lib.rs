#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks conventional Bevy plugin and system-set names.
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
use {bevy_app as _, bevy_ecs as _};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_UNCONVENTIONAL_NAMING,
    Warn,
    "a Bevy Plugin or SystemSet uses an unconventional name",
    BevyUnconventionalNaming
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyUnconventionalNaming {
    /// Check local Bevy trait targets for role-revealing suffixes.
    fn check_crate(&mut self, cx: &rustc_lint::LateContext<'tcx>) {
        for (target, trait_name, suffix) in bevy_support::unconventional_bevy_type_names(cx) {
            cx.emit_span_lint(
                BEVY_UNCONVENTIONAL_NAMING,
                cx.tcx.def_span(target),
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(format!(
                            "this `{trait_name}` name does not end in `{suffix}`"
                        ))
                        .help(format!("rename the type with the `{suffix}` suffix"));
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
