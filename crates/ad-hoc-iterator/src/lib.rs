#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc Iterator-style next methods.
//!
//! It identifies source-authored inherent methods that expose sequential access
//! through a custom `next`-shaped API without implementing the standard
//! `Iterator` contract. Resolved receiver and return types keep unrelated
//! accessors out of scope, and the trait solver skips types that already
//! implement `Iterator`.

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
use rustc_middle::ty;
use rustc_span::sym;
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_ITERATOR,
    Warn,
    "ad hoc next method could be `Iterator`",
    AdHocIterator
}

impl<'tcx> LateLintPass<'tcx> for AdHocIterator {
    /// Check one inherent method for the iterator-step shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        if iterator_candidate(cx, item).is_none() {
            return;
        }

        let name = item.ident.name;
        cx.emit_span_lint(
            AD_HOC_ITERATOR,
            item.span,
            DiagDecorator(move |diag| {
                let _ =
                    diag.primary_message(format!("method `{name}` looks like an iterator step"));
                let _ = diag
                    .help("implement `Iterator` when this is the canonical next item for the type");
            }),
        );
    }
}

/// Return the receiver type when an inherent method has the iterator-step shape.
fn iterator_candidate<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx ImplItem<'tcx>,
) -> Option<ty::Ty<'tcx>> {
    // Trait methods have names fixed by their trait, so only inherent methods are candidates.
    if !matches!(item.kind, ImplItemKind::Fn(..))
        || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        || !is_next_name(item.ident.name.as_str())
    {
        return None;
    }

    // Resolve the signature so aliases of `Option` count.
    let def_id = item.owner_id.def_id;
    let sig = cx
        .tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .skip_binder();
    let self_ty = cx
        .tcx
        .type_of(cx.tcx.local_parent(def_id))
        .instantiate_identity()
        .skip_norm_wip();
    // Require `&mut self` and an `Option<T>` whose item is not the type's own next state.
    let ty::Adt(option, args) = sig.output().kind() else {
        return None;
    };
    let item_ty = args.types().next()?;
    if !cx.tcx.is_diagnostic_item(sym::Option, option.did())
        || item_ty == self_ty
        || !cx.tcx.associated_item(def_id).is_method()
        || !matches!(sig.inputs(), [receiver] if matches!(receiver.kind(), ty::Ref(_, _, Mutability::Mut)))
    {
        return None;
    }

    // A type that already implements `Iterator` offers the standard step.
    let has_iterator_impl = has_iterator_impl(cx, self_ty);
    (!has_iterator_impl).then_some(self_ty)
}

/// Return whether the receiver already implements the standard iterator contract.
fn has_iterator_impl<'tcx>(cx: &LateContext<'tcx>, self_ty: ty::Ty<'tcx>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::Iterator)
        .is_none_or(|iterator| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(iterator, [self_ty], cx.param_env)
                .may_apply()
        })
}

/// Return whether the name reads as a `next` step without a domain-policy word.
fn is_next_name(name: &str) -> bool {
    (name == "next" || name.starts_with("next_")) && !domain_policy_name(name)
}

/// Return whether the name contains a word that marks a domain step rather than iteration.
fn domain_policy_name(name: &str) -> bool {
    [
        "advance",
        "backoff",
        "event",
        "page",
        "paged",
        "phase",
        "policy",
        "retry",
        "state",
        "status",
        "transition",
    ]
    .iter()
    .any(|word| name.contains(word))
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
