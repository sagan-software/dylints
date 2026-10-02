#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc IntoIterator-style methods.
//!
//! It inspects source-authored inherent methods whose names and signatures
//! expose iteration through a custom method rather than the standard
//! `IntoIterator` contract. The trait solver decides whether the returned type
//! is an iterator and whether the receiver type already implements
//! `IntoIterator`, so conversions such as `into_bytes` stay out of scope.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, ImplItemImplKind, ImplItemKind};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::Ty;
use rustc_span::{Symbol, sym};
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_INTO_ITERATOR,
    Warn,
    "ad hoc iteration method could be `IntoIterator`",
    AdHocIntoIterator
}

impl<'tcx> LateLintPass<'tcx> for AdHocIntoIterator {
    /// Check one inherent receiver method for the iteration-view shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Trait methods have names fixed by their trait, so only inherent methods are candidates.
        let def_id = item.owner_id.def_id;
        if !matches!(item.kind, ImplItemKind::Fn(..))
            || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            || !is_iteration_name(item.ident.name.as_str())
            || !cx.tcx.associated_item(def_id).is_method()
        {
            return;
        }

        // Erase late-bound lifetimes so the trait solver sees closed types.
        let sig = cx.tcx.instantiate_bound_regions_with_erased(
            cx.tcx.fn_sig(def_id).instantiate_identity().skip_norm_wip(),
        );
        let [receiver] = sig.inputs() else {
            return;
        };

        // Require an iterator result and a receiver type that lacks `IntoIterator`.
        if !has_trait_impl(cx, sig.output(), sym::Iterator)
            || has_trait_impl(cx, *receiver, sym::IntoIterator)
        {
            return;
        }

        let name = item.ident.name;
        let receiver = *receiver;
        cx.emit_span_lint(
            AD_HOC_INTO_ITERATOR,
            item.span,
            DiagDecorator(move |diag| {
                let _ = diag.primary_message(format!("method `{name}` looks like ad hoc iteration"));
                let _ = diag.help(format!(
                    "implement `IntoIterator for {receiver}` when this is the canonical iteration view"
                ));
            }),
        );
    }
}

/// Return whether `ty` may implement the diagnostic-item trait in the item's environment.
fn has_trait_impl<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>, trait_item: Symbol) -> bool {
    cx.tcx
        .get_diagnostic_item(trait_item)
        .is_some_and(|trait_def_id| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(trait_def_id, [ty], cx.param_env)
                .may_apply()
        })
}

/// Return whether the name reads as an iteration view without a domain-policy word.
fn is_iteration_name(name: &str) -> bool {
    (name.starts_with("into_") || name.starts_with("iter_") || name == "items")
        && !domain_policy_name(name)
}

/// Return whether the name contains a word that marks a filtered or reshaped view.
fn domain_policy_name(name: &str) -> bool {
    [
        "active",
        "allocated",
        "batched",
        "chunk",
        "chunked",
        "cloned",
        "dedup",
        "deduplicated",
        "enabled",
        "filter",
        "filtered",
        "matching",
        "owned",
        "page",
        "paged",
        "sort",
        "sorted",
        "unique",
        "valid",
        "visible",
    ]
    .iter()
    .any(|word| name.contains(word))
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
