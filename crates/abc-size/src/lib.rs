#![feature(rustc_private)]

//! A lint to bound per-function ABC size.
//!
//! It measures assignments, branches, and condition predicates in
//! source-authored callable bodies, then reports functions whose combined
//! magnitude exceeds the configured limit. Generated code and unsupported
//! macro expansions are excluded so the result reflects source-level design
//! complexity that a maintainer can act on.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{abc_size, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted ABC vector magnitude for one callable.
const ABC_SIZE_LIMIT: f64 = 25.0;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub ABC_SIZE,
    Warn,
    "function exceeds the ABC size limit",
    AbcSize
}

impl<'tcx> LateLintPass<'tcx> for AbcSize {
    /// Check one source-authored callable against the ABC magnitude limit.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Ignore generated callables because their source metric is not actionable.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || is_macro_expansion(span)
        {
            return;
        }

        // Preserve the full vector so the diagnostic identifies the dominant component.
        let size = abc_size(body);
        let magnitude = size.magnitude();
        if magnitude <= ABC_SIZE_LIMIT {
            return;
        }

        let assignments = size.assignments();
        let calls = size.calls();
        let conditions = size.conditions();
        // Report once at the callable boundary instead of on each contributing expression.
        cx.emit_span_lint(
            ABC_SIZE,
            span,
            DiagDecorator(move |diagnostic| {
                let _configured = diagnostic
                    .primary_message(format!(
                        "function has ABC size <{assignments}, {calls}, {conditions}> with magnitude {magnitude:.2}, which exceeds {ABC_SIZE_LIMIT:.2}"
                    ))
                    .help(
                        "split repeated state changes, calls, or conditions into a focused operation",
                    );
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
