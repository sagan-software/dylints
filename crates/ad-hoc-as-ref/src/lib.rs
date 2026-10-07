#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc AsRef-style methods.
//!
//! It inspects source-authored inherent methods whose names and signatures
//! expose a reference conversion without implementing the standard `AsRef`
//! or `AsMut` trait. Resolved receiver and return types keep unrelated
//! accessors and consuming conversions out of scope, and the trait solver
//! skips types that already implement the matching trait.

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
    pub AD_HOC_AS_REF,
    Warn,
    "ad hoc borrowed accessor could be `AsRef`",
    AdHocAsRef
}

impl<'tcx> LateLintPass<'tcx> for AdHocAsRef {
    /// Check one inherent method for the borrowed-accessor shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let Some((target, mutability)) = accessor_candidate(cx, item) else {
            return;
        };
        let def_id = item.owner_id.def_id;

        // `AsMut` is the matching trait for a mutable view.
        let (trait_name, trait_item) = match mutability {
            Mutability::Not => ("AsRef", sym::AsRef),
            Mutability::Mut => ("AsMut", sym::AsMut),
        };
        let self_ty = cx
            .tcx
            .type_of(cx.tcx.local_parent(def_id))
            .instantiate_identity()
            .skip_norm_wip();
        // An existing implementation already offers the standard view.
        let is_already_implemented = cx
            .tcx
            .get_diagnostic_item(trait_item)
            .is_none_or(|trait_def_id| has_trait_implementation(cx, self_ty, trait_def_id, target));
        if is_already_implemented {
            return;
        }

        let name = item.ident.name;
        cx.emit_span_lint(
            AD_HOC_AS_REF,
            item.span,
            DiagDecorator(move |diag| {
                let _ = diag.primary_message(format!(
                    "method `{name}` looks like an `{trait_name}` accessor"
                ));
                let _ = diag.help(format!(
                    "implement `{trait_name}<{target}>` when this exposes the canonical borrowed view"
                ));
            }),
        );
    }
}

/// Return the resolved target and mutability for an inherent accessor candidate.
fn accessor_candidate<'tcx>(
    cx: &LateContext<'tcx>,
    item: &ImplItem<'tcx>,
) -> Option<(Ty<'tcx>, Mutability)> {
    // Trait methods have names fixed by their trait, so only inherent methods are candidates.
    if !matches!(item.kind, ImplItemKind::Fn(..))
        || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        || !is_accessor_name(item.ident.name.as_str())
    {
        return None;
    }

    // Read the lowered signature so aliases such as `type PathRef<'a> = &'a Path` count.
    let def_id = item.owner_id.def_id;
    let sig = cx.tcx.instantiate_bound_regions_with_erased(
        cx.tcx.fn_sig(def_id).instantiate_identity().skip_norm_wip(),
    );
    let ([receiver], ty::Ref(_, target, mutability)) = (sig.inputs(), sig.output().kind()) else {
        return None;
    };
    (cx.tcx.associated_item(def_id).is_method() && receiver.is_ref())
        .then_some((*target, *mutability))
}

/// Return whether the method name reads as a plain accessor.
fn is_accessor_name(name: &str) -> bool {
    (name.starts_with("as_") || name.starts_with("get_")) && !name.starts_with("get_or_")
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
