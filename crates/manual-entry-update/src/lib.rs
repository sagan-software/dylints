#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

//! A lint for direct standard map `Entry` update matches.
//!
//! It resolves the matched type to the standard `HashMap` or `BTreeMap`
//! `Entry`, resolves each arm pattern to its `Occupied` or `Vacant` variant,
//! and resolves the update and insertion calls to the entry types' own
//! methods. Each arm must use its entry binding exactly once, so the arm can
//! move into an `and_modify` closure or an `or_insert_with` value.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    Arm, BindingMode, ByRef, Expr, ExprKind, HirId, MatchSource, PatKind,
    def::Res,
    def_id::DefId,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    hir::nested_filter,
    ty::{self, TyCtxt},
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ENTRY_UPDATE,
    Warn,
    "standard map entry match can use `and_modify(...).or_insert_with(...)`",
    ManualEntryUpdate
}

impl<'tcx> LateLintPass<'tcx> for ManualEntryUpdate {
    /// Checks one match over a standard `HashMap` or `BTreeMap` entry.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if is_entry_update_candidate(cx, expr) {
            // Report only the complete two-arm value update that has a chained equivalent.
            support::emit(
                cx,
                MANUAL_ENTRY_UPDATE,
                expr.span,
                "standard map entry match can use `and_modify(...).or_insert_with(...)`",
                "use `and_modify(...).or_insert_with(...)` for this value-only entry update",
            );
        }
    }
}

/// Return whether one match has the supported standard-entry update shape.
fn is_entry_update_candidate<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> bool {
    // Establish a unit-typed two-arm match over a standard `Entry`.
    let ExprKind::Match(scrutinee, [first_arm, second_arm], MatchSource::Normal) = expr.kind else {
        return false;
    };
    let typeck = cx.typeck_results();
    let ty::Adt(entry, _) = typeck.expr_ty(scrutinee).kind() else {
        return false;
    };
    let is_unit = typeck.expr_ty(expr).is_unit();
    let is_standard_entry = cx
        .tcx
        .get_diagnostic_name(entry.did())
        .is_some_and(|name| matches!(name.as_str(), "HashMapEntry" | "BTreeEntry"));

    // Resolve both arms to the two variants. An exhaustive two-arm match without
    // guards names each variant once, so the occupied arm decides the order.
    let arms = entry_arm(cx, first_arm, entry.did())
        .zip(entry_arm(cx, second_arm, entry.did()))
        .map(|(first, second)| {
            if first.is_occupied {
                (first, second)
            } else {
                (second, first)
            }
        });

    // The occupied arm becomes an `and_modify` closure; the vacant arm supplies one value.
    is_unit
        && is_standard_entry
        && arms.is_some_and(|(occupied, vacant)| {
            is_occupied_update(cx, occupied) && is_vacant_insert(cx, vacant)
        })
}

/// One unguarded arm that binds the payload of an `Entry` variant.
#[derive(Clone, Copy)]
struct EntryArm<'tcx> {
    /// Whether the arm matches the `Occupied` variant rather than `Vacant`.
    is_occupied: bool,
    /// Binding for the variant's entry payload.
    binding: HirId,
    /// Arm body with a single-expression block peeled.
    body: &'tcx Expr<'tcx>,
}

/// Resolves an unguarded `Entry::Variant(binding)` arm.
fn entry_arm<'tcx>(
    cx: &LateContext<'tcx>,
    arm: &'tcx Arm<'tcx>,
    entry: DefId,
) -> Option<EntryArm<'tcx>> {
    // Resolve the constructor path to a variant of the matched `Entry` type.
    let PatKind::TupleStruct(qpath, [inner], _) = arm.pat.kind else {
        return None;
    };
    let ctor = cx.qpath_res(&qpath, arm.pat.hir_id).opt_def_id()?;
    let variant = cx.tcx.parent(ctor);

    // Require a by-value payload binding and no guard, so the arm body can move.
    let PatKind::Binding(BindingMode(ByRef::No, _), binding, _, None) = inner.kind else {
        return None;
    };
    (arm.guard.is_none() && cx.tcx.parent(variant) == entry).then(|| EntryArm {
        is_occupied: cx.tcx.item_name(variant).as_str() == "Occupied",
        binding,
        body: peel_block(arm.body),
    })
}

/// Returns whether the occupied arm only updates the value through one entry call.
fn is_occupied_update<'tcx>(cx: &LateContext<'tcx>, arm: EntryArm<'tcx>) -> bool {
    let uses = EntryUses::collect(cx, arm.binding, arm.body);
    uses.total == 1 && uses.updates == 1 && !uses.has_control_flow
}

/// Returns whether the vacant arm is one insertion that does not reuse the entry.
fn is_vacant_insert<'tcx>(cx: &LateContext<'tcx>, arm: EntryArm<'tcx>) -> bool {
    let ExprKind::MethodCall(_, receiver, [value], _) = arm.body.kind else {
        return false;
    };
    let uses = EntryUses::collect(cx, arm.binding, value);
    local_id(cx, receiver) == Some(arm.binding)
        && entry_method(cx, arm.body, arm.binding).is_some_and(|name| name.as_str() == "insert")
        && uses.total == 0
        && !uses.has_control_flow
}

/// Returns the name of an entry type's own method called on the entry binding.
fn entry_method(
    cx: &LateContext<'_>,
    call: &Expr<'_>,
    binding: HirId,
) -> Option<rustc_span::Symbol> {
    // Compare the method's inherent owner with the binding's resolved entry type.
    let typeck = cx.typeck_results();
    let method = typeck.type_dependent_def_id(call.hir_id)?;
    let owner = cx.tcx.inherent_impl_of_assoc(method)?;
    let owner_adt = cx
        .tcx
        .type_of(owner)
        .instantiate_identity()
        .skip_normalization()
        .ty_adt_def()?;
    (typeck.node_type(binding).ty_adt_def() == Some(owner_adt)).then(|| cx.tcx.item_name(method))
}

/// Removes a block wrapper that holds exactly one expression.
fn peel_block<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::Block(block, None) => support::block_only_expr(block).unwrap_or(expr),
        _ => expr,
    }
}

/// Returns the local binding resolved by a path expression.
fn local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };
    match cx.qpath_res(&qpath, expr.hir_id) {
        Res::Local(id) => Some(id),
        _ => None,
    }
}

/// Counts uses of one entry binding and records closure-hostile control flow.
struct EntryUses<'cx, 'tcx> {
    /// Compiler context used for resolution.
    cx: &'cx LateContext<'tcx>,
    /// Entry binding whose uses are counted.
    binding: HirId,
    /// Number of path uses of the binding.
    total: usize,
    /// Number of `get_mut` or `into_mut` calls on the binding.
    updates: usize,
    /// Whether the expression can leave an enclosing closure.
    has_control_flow: bool,
}

impl<'cx, 'tcx> EntryUses<'cx, 'tcx> {
    /// Walks one expression and returns the counts.
    fn collect(cx: &'cx LateContext<'tcx>, binding: HirId, expr: &'tcx Expr<'tcx>) -> Self {
        let mut uses = Self {
            cx,
            binding,
            total: 0,
            updates: 0,
            has_control_flow: false,
        };
        uses.visit_expr(expr);
        uses
    }
}

impl<'tcx> Visitor<'tcx> for EntryUses<'_, 'tcx> {
    /// Closures in an arm can also capture the entry.
    type NestedFilter = nested_filter::OnlyBodies;

    /// Returns the type context used to enter closure bodies.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Counts entry uses and value-update calls, and records control flow.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        match expr.kind {
            // Every path to the binding is one use, including a method receiver.
            ExprKind::Path(_) if local_id(self.cx, expr) == Some(self.binding) => self.total += 1,
            ExprKind::MethodCall(_, receiver, [], _)
                if local_id(self.cx, receiver) == Some(self.binding) =>
            {
                // Count only the entry type's own value accessors as updates.
                let name = entry_method(self.cx, expr, self.binding);
                if name.is_some_and(|name| matches!(name.as_str(), "get_mut" | "into_mut")) {
                    self.updates += 1;
                }
            }
            ExprKind::Ret(_)
            | ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Become(_)
            | ExprKind::Yield(..)
            | ExprKind::Match(_, _, MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar) => {
                self.has_control_flow = true;
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
