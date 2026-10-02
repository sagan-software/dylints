#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for ad hoc Borrow-style methods.
//!
//! It identifies source-authored inherent methods that expose a borrowed view
//! through a custom name instead of the standard `Borrow` trait. Resolved
//! receiver and return types keep unrelated accessors and consuming conversions
//! outside the recommendation, and the trait solver skips types that already
//! implement `Borrow` for the returned type.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, ImplItemImplKind, ImplItemKind, Mutability};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{def_id::DefId, sym};
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_BORROW,
    Warn,
    "ad hoc borrowed accessor could be `Borrow`",
    AdHocBorrow
}

impl<'tcx> LateLintPass<'tcx> for AdHocBorrow {
    /// Check one inherent method for the shared borrowed-view shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait methods have names fixed by their trait, so only inherent methods are candidates.
        if !matches!(item.kind, ImplItemKind::Fn(..))
            || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            || !item.ident.name.as_str().starts_with("borrow_")
        {
            return;
        }

        // Resolve the signature so aliases such as `type NameRef<'a> = &'a str` count.
        let def_id = item.owner_id.def_id;
        let sig = cx.tcx.instantiate_bound_regions_with_erased(
            cx.tcx.fn_sig(def_id).instantiate_identity().skip_norm_wip(),
        );
        let ([receiver], ty::Ref(_, target, Mutability::Not)) = (sig.inputs(), sig.output().kind())
        else {
            return;
        };
        if !cx.tcx.associated_item(def_id).is_method()
            || !matches!(receiver.kind(), ty::Ref(_, _, Mutability::Not))
        {
            return;
        }

        // An existing `Borrow<target>` implementation already offers the standard view.
        let self_ty = cx
            .tcx
            .type_of(cx.tcx.local_parent(def_id))
            .instantiate_identity()
            .skip_norm_wip();
        if cx
            .tcx
            .get_diagnostic_item(sym::Borrow)
            .is_none_or(|borrow| has_trait_implementation(cx, self_ty, borrow, *target))
        {
            return;
        }

        let name = item.ident.name;
        let target = *target;
        cx.emit_span_lint(
            AD_HOC_BORROW,
            item.span,
            DiagDecorator(move |diag| {
                let _ = diag.primary_message(format!("method `{name}` looks like a `Borrow` accessor"));
                let _ = diag.help(format!(
                    "implement `std::borrow::Borrow<{target}>` only when equality and hashing match the borrowed view"
                ));
            }),
        );
    }
}

/// Return whether `self_ty: Trait<target>` holds in the item's environment.
fn has_trait_implementation<'tcx>(
    cx: &LateContext<'tcx>,
    self_ty: Ty<'tcx>,
    trait_def_id: DefId,
    target: Ty<'tcx>,
) -> bool {
    cx.tcx
        .infer_ctxt()
        .build(cx.typing_mode())
        .type_implements_trait(trait_def_id, [self_ty, target], cx.param_env)
        .must_apply_modulo_regions()
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
