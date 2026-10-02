#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks zero-sized Bevy components fetched as query data.
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
use bevy_ecs as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_ZST_QUERY,
    Warn,
    "a Bevy query fetches a zero-sized component",
    BevyZstQuery
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyZstQuery {
    /// Check function parameters for zero-sized query data.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        _: &'tcx rustc_hir::Body<'tcx>,
        _: rustc_span::Span,
        local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        // Map each zero-sized query parameter index back to its declaration span.
        let indexes = bevy_support::zst_query_parameters(cx, kind, local_def_id);
        for span in bevy_support::parameter_spans(declaration, indexes) {
            cx.emit_span_lint(
                BEVY_ZST_QUERY,
                span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this query fetches a zero-sized component")
                        .help("move marker components to a `With<T>` or `Without<T>` filter");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
