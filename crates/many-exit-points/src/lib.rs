#![feature(rustc_private)]

//! A lint to bound explicit and desugared exits per callable.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use maintainability_support::{exit_point_count, is_macro_expansion};
use rustc_errors::DiagDecorator;
use rustc_hir::{Body, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

/// Largest accepted number of explicit or `?`-desugared exits.
const EXIT_POINT_LIMIT: u32 = 4;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANY_EXIT_POINTS,
    Warn,
    "function exceeds the exit-point limit",
    ManyExitPoints
}

impl<'tcx> LateLintPass<'tcx> for ManyExitPoints {
    /// Check one source-authored callable for too many exits.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _kind: FnKind<'tcx>,
        _declaration: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Ignore generated callables because their exit structure is not user-controlled.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || is_macro_expansion(span)
        {
            return;
        }

        // Count the complete body before deciding whether one callable-level warning applies.
        let exits = exit_point_count(body);
        if exits <= EXIT_POINT_LIMIT {
            return;
        }

        // Anchor the diagnostic on the callable rather than one arbitrary exit.
        cx.emit_span_lint(
            MANY_EXIT_POINTS,
            span,
            DiagDecorator(move |diagnostic| {
                let _configured = diagnostic
                    .primary_message(format!(
                        "function has {exits} explicit or desugared exit points, which exceeds {EXIT_POINT_LIMIT}"
                    ))
                    .help("keep useful guard clauses and extract independent fallible stages");
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
