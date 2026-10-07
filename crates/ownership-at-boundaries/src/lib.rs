#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for owned `String` and `Vec` parameters at API boundaries.
//!
//! It inspects `pub` function signatures whose signature no trait fixes, finds
//! parameters that take a `String` or `Vec` by value, and runs rustc's
//! expression-use analysis over the body. A parameter is reported only when the
//! body never moves it and never mutates it, so every use could borrow instead.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_hir_typeck;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{BindingMode, Body, FnDecl, HirId, LangItem, PatKind, intravisit::FnKind};
use rustc_hir_typeck::expr_use_visitor::{Delegate, ExprUseVisitor, PlaceBase, PlaceWithHirId};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{
    mir::FakeReadCause,
    ty::{self, BorrowKind, Ty},
};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub OWNERSHIP_AT_BOUNDARIES,
    Warn,
    "owned `String` or `Vec` parameter at an API boundary",
    OwnershipAtBoundaries
}

impl<'tcx> LateLintPass<'tcx> for OwnershipAtBoundaries {
    /// Check each `pub` function for owned parameters that are only borrowed.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Filter out closures, generated code, trait methods, and async functions before analysis.
        if matches!(kind, FnKind::Closure)
            || span.from_expansion()
            || !is_free_public_signature(cx, local_def_id)
        {
            return;
        }

        // Collect immutable by-value bindings of owned `String` or `Vec` parameters.
        let candidates: Vec<(HirId, Span, &'static str)> = decl
            .inputs
            .iter()
            .zip(body.params)
            .filter_map(|(source_ty, param)| {
                let PatKind::Binding(BindingMode::NONE, binding_id, _ident, None) = param.pat.kind
                else {
                    return None;
                };
                let kind = owned_boundary_kind(cx, cx.typeck_results().node_type(param.hir_id))?;
                Some((binding_id, source_ty.span, kind))
            })
            .collect();
        if candidates.is_empty() {
            return;
        }

        // Record every binding the body moves or mutates; those uses need ownership.
        let mut uses = OwnershipUses::default();
        let Ok(()) = ExprUseVisitor::for_clippy(cx, local_def_id, &mut uses).consume_body(body);

        for (binding_id, span, kind) in candidates {
            if uses.owned.contains(&binding_id) {
                continue;
            }
            cx.emit_span_lint(
                OWNERSHIP_AT_BOUNDARIES,
                span,
                DiagDecorator(|diag| {
                    let _ = diag.primary_message(format!(
                        "this parameter takes owned `{kind}` but the function only borrows it"
                    ));
                    let _ = diag.help(if kind == "String" {
                        "take `&str` instead"
                    } else {
                        "take a slice `&[T]` instead"
                    });
                }),
            );
        }
    }
}

/// Return whether the function is a `pub`, non-`async` function that no trait fixes.
///
/// An `async fn` moves every parameter into its future, which hides its uses.
fn is_free_public_signature(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    // Public visibility defines the API boundary this lint evaluates.
    let def_id = local_def_id.to_def_id();
    let is_public = cx.tcx.visibility(local_def_id).is_public();
    // Trait methods and async functions have ownership semantics outside this check.
    let is_trait_item = cx.tcx.trait_of_assoc(def_id).is_some();
    let is_trait_impl_item = cx.tcx.trait_impl_of_assoc(def_id).is_some();
    let is_async = cx.tcx.asyncness(local_def_id).is_async();

    is_public && !is_trait_item && !is_trait_impl_item && !is_async
}

/// Locals whose uses require ownership, collected from rustc's expression-use analysis.
#[derive(Default)]
struct OwnershipUses {
    /// Bindings that are moved as a whole or borrowed mutably.
    owned: Vec<HirId>,
}

impl OwnershipUses {
    /// Record the local behind a place.
    fn record(&mut self, place_with_id: &PlaceWithHirId<'_>) {
        if let PlaceBase::Local(hir_id) = place_with_id.place.base {
            self.owned.push(hir_id);
        }
    }
}

impl<'tcx> Delegate<'tcx> for OwnershipUses {
    /// A move of the whole value, or a capture by value, needs ownership.
    fn consume(&mut self, place_with_id: &PlaceWithHirId<'tcx>, _diag_expr_id: HirId) {
        if place_with_id.place.projections.is_empty() {
            self.record(place_with_id);
        }
    }

    /// An explicit `.use` clone only reads its operand.
    fn use_cloned(&mut self, _place_with_id: &PlaceWithHirId<'tcx>, _diag_expr_id: HirId) {}

    /// A mutable or unique borrow needs ownership; a shared borrow does not.
    fn borrow(
        &mut self,
        place_with_id: &PlaceWithHirId<'tcx>,
        _diag_expr_id: HirId,
        bk: BorrowKind,
    ) {
        if !matches!(bk, BorrowKind::Immutable) {
            self.record(place_with_id);
        }
    }

    /// An assignment through the place needs ownership.
    fn mutate(&mut self, assignee_place: &PlaceWithHirId<'tcx>, _diag_expr_id: HirId) {
        self.record(assignee_place);
    }

    /// Initializing the parameter binding itself is not a use.
    fn bind(&mut self, _binding_place: &PlaceWithHirId<'tcx>, _diag_expr_id: HirId) {}

    /// Reads for pattern matching only inspect the value.
    fn fake_read(
        &mut self,
        _place_with_id: &PlaceWithHirId<'tcx>,
        _cause: FakeReadCause,
        _diag_expr_id: HirId,
    ) {
    }
}

/// Return the reported name when the type is the standard `String` or `Vec`.
fn owned_boundary_kind(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<&'static str> {
    // Match the resolved standard types, so aliases count and lookalikes do not.
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };

    if cx.tcx.is_lang_item(adt.did(), LangItem::String) {
        Some("String")
    } else {
        cx.tcx
            .is_diagnostic_item(sym::Vec, adt.did())
            .then_some("Vec")
    }
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
