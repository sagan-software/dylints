#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks borrowed Bevy proxy parameters that support reborrowing.
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
    pub BEVY_BORROWED_REBORROWABLE,
    Warn,
    "a reborrowable Bevy proxy is taken through another mutable reference",
    BevyBorrowedReborrowable
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyBorrowedReborrowable {
    /// Check function parameters for needless mutable-reference wrappers.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: rustc_span::Span,
        local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        // Report each borrowed reborrowable parameter at its declaration span.
        for (index, proxy) in
            bevy_support::borrowed_reborrowable_parameters(cx, kind, body, local_def_id)
        {
            let Some(parameter) = declaration.inputs.get(index) else {
                continue;
            };
            cx.emit_span_lint(
                BEVY_BORROWED_REBORROWABLE,
                parameter.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(format!(
                            "Bevy `{}` already supports reborrowing",
                            proxy.name()
                        ))
                        .help("take the proxy by value and call its reborrow method where needed");
                }),
            );
        }
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
