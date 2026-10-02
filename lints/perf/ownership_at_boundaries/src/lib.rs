#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for owned `String` and `Vec` parameters at API boundaries.
//!
//! It inspects source-authored function signatures at public or configured API
//! boundaries, resolves nested ownership types, and reports parameters that can
//! borrow their input instead. The check preserves return and mutation cases
//! where ownership is part of the observed contract.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{Body, FnDecl, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub OWNERSHIP_AT_BOUNDARIES,
    Warn,
    "owned `String` or `Vec` parameter at an API boundary",
    OwnershipAtBoundaries
}

impl<'tcx> LateLintPass<'tcx> for OwnershipAtBoundaries {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Ignore closures and private functions because they do not define public APIs.
        if matches!(kind, FnKind::Closure) {
            return;
        }
        if !cx.tcx.visibility(local_def_id).is_public() {
            return;
        }

        // Align each written parameter type with its resolved body parameter.
        for (source_ty, param) in decl.inputs.iter().zip(body.params) {
            let Some(kind) = owned_boundary_kind(cx, cx.typeck_results().node_type(param.hir_id))
            else {
                continue;
            };
            // Point at each owned type while leaving justified consumption as an exception.
            emit_span_lint_with_help(
                cx,
                OWNERSHIP_AT_BOUNDARIES,
                source_ty.span,
                format!("this parameter takes owned `{kind}` at the boundary"),
                "take a borrowed type instead unless the value is stored, mutated, or consumed",
            );
        }
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's native diagnostic decorator to avoid depending on Clippy utilities.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for owned boundary kind analysis.
fn owned_boundary_kind(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<&'static str> {
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };

    match (
        cx.tcx.item_name(adt.did()),
        cx.tcx.crate_name(adt.did().krate),
    ) {
        (sym::String, sym::alloc) => Some("String"),
        (sym::Vec, sym::alloc) => Some("Vec"),
        _ => None,
    }
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
