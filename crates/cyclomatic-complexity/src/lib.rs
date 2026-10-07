#![feature(rustc_private)]

//! A lint to bound source-authored Cyclomatic Complexity.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{cyclomatic_complexity, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted per-callable Cyclomatic Complexity.
const COMPLEXITY_LIMIT: u32 = 9;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CYCLOMATIC_COMPLEXITY,
    Warn,
    "function exceeds the Cyclomatic Complexity limit",
    CyclomaticComplexity
}

impl<'tcx> LateLintPass<'tcx> for CyclomaticComplexity {
    /// Check one callable body for excessive path complexity.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Do not charge a generated callable to the source item that invoked its macro.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || is_macro_expansion(span)
        {
            return;
        }

        // Measure only source-authored bodies after generated and support code is excluded.
        let complexity = cyclomatic_complexity(body);
        if complexity <= COMPLEXITY_LIMIT {
            return;
        }

        cx.emit_span_lint(
            CYCLOMATIC_COMPLEXITY,
            span,
            DiagDecorator(move |diag| {
                let _configured = diag
                    .primary_message(format!(
                        "function has Cyclomatic Complexity {complexity}, which exceeds {COMPLEXITY_LIMIT}"
                    ))
                    .help(
                        "split independent decisions into smaller functions or typed state transitions",
                    );
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
