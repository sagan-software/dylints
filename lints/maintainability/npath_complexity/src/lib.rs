#![feature(rustc_private)]

//! A lint to bound estimated acyclic execution paths.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{is_macro_expansion, npath_complexity};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted per-callable `NPath` estimate.
const NPATH_LIMIT: u128 = 200;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub NPATH_COMPLEXITY,
    Warn,
    "function exceeds the NPath complexity limit",
    NPathComplexity
}

impl<'tcx> LateLintPass<'tcx> for NPathComplexity {
    /// Check one callable body for combinatorial route growth.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Ignore generated or internal support code before measuring source complexity.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || is_macro_expansion(span)
        {
            return;
        }
        // Emit only when the measured route count exceeds the configured bound.
        let complexity = npath_complexity(body);
        if complexity <= NPATH_LIMIT {
            return;
        }
        cx.emit_span_lint(
            NPATH_COMPLEXITY,
            span,
            DiagDecorator(move |diagnostic| {
                let _configured = diagnostic
                    .primary_message(format!(
                        "function has NPath complexity {complexity}, which exceeds {NPATH_LIMIT}"
                    ))
                    .help("split independent branching stages into separate functions");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
