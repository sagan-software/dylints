#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks mutable Bevy queries used only for reading.
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
    pub BEVY_READONLY_SYSTEM_ACCESS,
    Warn,
    "a mutable Bevy query is used only for reading",
    BevyReadonlySystemAccess
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyReadonlySystemAccess {
    /// Check function parameters for unnecessary mutable query access.
    fn check_fn(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        kind: rustc_hir::intravisit::FnKind<'tcx>,
        declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx rustc_hir::Body<'tcx>,
        _: rustc_span::Span,
        local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        // Report each mutable query used only for reads at its parameter declaration.
        let indexes = bevy_support::readonly_mut_query_parameters(cx, kind, body, local_def_id);
        for parameter in bevy_support::parameter_types(declaration, indexes) {
            let replacements = bevy_support::shared_query_data_replacements(cx, parameter);
            cx.emit_span_lint(
                BEVY_READONLY_SYSTEM_ACCESS,
                parameter.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic =
                        diagnostic.primary_message("this mutable query is used only for reading");
                    // Callers that pass this exact query type would stop compiling, so the
                    // rewrite is offered for review rather than applied automatically.
                    if replacements.is_empty() {
                        let _helped =
                            diagnostic.help("request shared component references in this query");
                    } else {
                        let _suggested = diagnostic.multipart_suggestion(
                            "request shared component references in this query",
                            replacements,
                            rustc_errors::Applicability::MaybeIncorrect,
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
