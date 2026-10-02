#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

//! A lint for direct standard map `Entry` update matches.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};

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
fn is_entry_update_candidate(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Establish the standard `Entry` type before inspecting the arm shapes.
    match expr.kind {
        ExprKind::Match(scrutinee, arms, _) => Some((scrutinee, arms)),
        _ => None,
    }
    .filter(|(scrutinee, arms)| {
        arms.len() == 2 && standard_entry_type(cx, cx.typeck_results().expr_ty(scrutinee))
    })
    .and_then(|_| support::snippet(cx, expr.span))
    .is_some_and(|source| {
        let has_value_update = source.contains(".get_mut()") || source.contains(".into_mut()");
        let has_both_arms = source.contains("Occupied") && source.contains("Vacant");
        let has_one_insert = source.matches(".insert(").count() == 1;
        let has_one_update =
            source.matches(".get_mut()").count() + source.matches(".into_mut()").count() == 1;
        let has_one_statement = source.matches(';').count() == 1;
        has_both_arms && has_value_update && has_one_insert && has_one_update && has_one_statement
    })
}

/// Returns whether a type is a standard hash-map or B-tree-map entry.
fn standard_entry_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };
    let path = cx.tcx.def_path_str(adt.did());
    path.contains("collections::hash_map::Entry") || path.contains("collections::btree_map::Entry")
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
