#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for tracing span guards held across await points.
//!
//! This Dylint library resolves tracing guard lifetimes, reports guards held
//! across await points, and recommends releasing the guard before suspension.
//!
//! The README defines the supported async shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;

#[cfg(test)]
use tracing as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Closure, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, def_id::DefId,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{mir::CoroutineLayout, ty::Adt};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRACING_AWAIT_HOLDING_SPAN_GUARD,
    Warn,
    "a tracing span guard is held across an await point",
    TracingAwaitHoldingSpanGuard
}

impl<'tcx> LateLintPass<'tcx> for TracingAwaitHoldingSpanGuard {
    /// Check the compiler-derived saved locals for each async body.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Restrict MIR inspection to compiler-desugared async coroutine bodies.
        let ExprKind::Closure(Closure {
            def_id,
            kind: ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _)),
            ..
        }) = expr.kind
        else {
            return;
        };
        let Some(coroutine_layout) = cx.tcx.mir_coroutine_witnesses(*def_id) else {
            return;
        };

        check_interior_types(cx, coroutine_layout);
    }
}

/// Diagnose tracing guards stored in coroutine states at suspension points.
fn check_interior_types(cx: &LateContext<'_>, coroutine: &CoroutineLayout<'_>) {
    // Inspect only saved ADT locals because tracing guards are ADTs.
    for (type_index, type_cause) in coroutine.field_tys.iter_enumerated() {
        let Adt(adt, _) = type_cause.ty.kind() else {
            continue;
        };
        if !is_tracing_span_guard(cx, adt.did()) {
            continue;
        }

        // Collect every suspension point whose coroutine state contains this guard.
        let await_points = coroutine
            .variant_source_info
            .iter_enumerated()
            .filter_map(|(variant, source_info)| {
                coroutine
                    .variant_fields
                    .get(variant)
                    .is_some_and(|fields| fields.raw.contains(&type_index))
                    .then_some(source_info.span)
            })
            .collect::<Vec<_>>();

        cx.emit_span_lint(
            TRACING_AWAIT_HOLDING_SPAN_GUARD,
            type_cause.source_info.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this tracing span guard is held across an `.await` point")
                    .span_note(
                        await_points,
                        "the span remains entered through these suspension points",
                    )
                    .help("use `Span::in_scope`, `Future::instrument`, or `#[instrument]`");
            }),
        );
    }
}

/// Prove that a saved local is one of tracing's two span guard types.
fn is_tracing_span_guard(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "tracing"
        && matches!(
            cx.tcx.def_path_str(def_id).as_str(),
            "tracing::span::Entered" | "tracing::span::EnteredSpan"
        )
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
