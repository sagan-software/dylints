#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc Iterator-style next methods.
//!
//! It identifies source-authored inherent methods that expose sequential access
//! through a custom `next`-shaped API without implementing the standard
//! `Iterator` contract. Resolved receiver, return, and mutation behavior keep
//! unrelated accessors and stateful methods outside the recommendation.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, ImplItemImplKind, ImplItemKind, Mutability};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_ITERATOR,
    Warn,
    "ad hoc next method could be `Iterator`",
    AdHocIterator
}

impl<'tcx> LateLintPass<'tcx> for AdHocIterator {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(_, _) = item.kind else {
            return;
        };

        // Exclude the canonical trait method and methods supplied by another trait.
        let name = item.ident.name.to_ident_string();
        if standard_iterator_method(cx, item, &name)
            || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        {
            return;
        }

        // Resolve the method signature so aliases do not affect the shape check.
        let sig = cx
            .tcx
            .fn_sig(item.owner_id.def_id)
            .instantiate_identity()
            .skip_binder();

        if next_name(&name)
            && mutable_receiver_only(sig.inputs())
            && iterator_item_return(cx, item, sig.output())
        {
            emit_span_lint_with_help(
                cx,
                AD_HOC_ITERATOR,
                item.span,
                format!("method `{name}` looks like an iterator step"),
                "implement `Iterator` when this is the canonical next item for the type",
            );
        }
    }
}

/// Return whether this is the standard iterator method shape.
fn standard_iterator_method(cx: &LateContext<'_>, item: &ImplItem<'_>, name: &str) -> bool {
    if name != "next" || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Query the parent impl so qualified paths and imports resolve by trait identity rather than
    // by the spelling used in the impl header.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    iterator_trait(cx, trait_def_id)
}

/// Helper for iterator trait analysis.
fn iterator_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    if cx.tcx.is_diagnostic_item(sym::Iterator, trait_def_id) {
        return true;
    }

    matches!(
        cx.tcx.def_path_str(trait_def_id).as_str(),
        "core::iter::traits::iterator::Iterator" | "std::iter::Iterator"
    )
}

/// Return the next name around a source position.
fn next_name(name: &str) -> bool {
    (name == "next" || name.starts_with("next_")) && !domain_policy_name(name)
}

/// Return the domain policy name.
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

/// Helper for mutable receiver only analysis.
fn mutable_receiver_only(inputs: &[Ty<'_>]) -> bool {
    matches!(inputs, [receiver] if mutable_ref(*receiver))
}

/// Helper for mutable ref analysis.
fn mutable_ref(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(_, _, Mutability::Mut))
}

/// Helper for iterator item return analysis.
fn iterator_item_return<'tcx>(cx: &LateContext<'tcx>, item: &ImplItem<'tcx>, ty: Ty<'tcx>) -> bool {
    let Some(item_ty) = option_item_ty(cx, ty) else {
        return false;
    };

    !returns_self_state(cx, item, item_ty)
}

/// Return type information for option item.
fn option_item_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };

    // Compare the resolved ADT against rustc's `Option` diagnostic item so aliases,
    // `std::option::Option`, and imported spellings are recognized while local lookalikes are not.
    if !cx.tcx.is_diagnostic_item(sym::Option, adt.did()) || args.len() != 1 {
        return None;
    }

    Some(args.type_at(0))
}

/// Return whether the item returns self state.
fn returns_self_state<'tcx>(
    cx: &LateContext<'tcx>,
    item: &ImplItem<'tcx>,
    item_ty: Ty<'tcx>,
) -> bool {
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();

    item_ty == self_ty
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

    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
