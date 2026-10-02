#![feature(rustc_private)]

//! A lint to bound source Cognitive Complexity.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{cognitive_complexity, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted per-callable source Cognitive Complexity.
const COMPLEXITY_LIMIT: u32 = 14;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SOURCE_COGNITIVE_COMPLEXITY,
    Warn,
    "function exceeds the source Cognitive Complexity limit",
    SourceCognitiveComplexity
}

impl<'tcx> LateLintPass<'tcx> for SourceCognitiveComplexity {
    /// Check one callable body for nested source control flow.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Exclude generated and internal helper code before measuring source complexity.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || is_macro_expansion(span)
        {
            return;
        }
        // Compute the complete body score before comparing it with the configured limit.
        let complexity = cognitive_complexity(body);
        if complexity <= COMPLEXITY_LIMIT {
            return;
        }
        cx.emit_span_lint(
            SOURCE_COGNITIVE_COMPLEXITY,
            span,
            DiagDecorator(move |diagnostic| {
                let _configured = diagnostic
                    .primary_message(format!(
                        "function has source Cognitive Complexity {complexity}, which exceeds {COMPLEXITY_LIMIT}"
                    ))
                    .help("flatten nested control flow or extract a named operation");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
