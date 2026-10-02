#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc display formatting methods.
//!
//! It finds inherent methods that return owned strings through common display-like
//! names, skipping methods with arguments and types that already implement
//! `Display`. The diagnostic points at the method declaration and recommends the
//! standard trait when the returned text is the type's canonical representation.

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
    pub AD_HOC_DISPLAY,
    Warn,
    "ad hoc display method could be `Display`",
    AdHocDisplay
}

impl<'tcx> LateLintPass<'tcx> for AdHocDisplay {
    /// Check one inherent method for the display-formatting shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        if !is_display_candidate(cx, item) {
            return;
        }

        let name = item.ident.name;
        cx.emit_span_lint(
            AD_HOC_DISPLAY,
            item.span,
            DiagDecorator(move |diag| {
                let _ = diag.primary_message(format!(
                    "method `{name}` looks like ad hoc display formatting"
                ));
                let _ = diag.help(
                    "implement `std::fmt::Display` when this is the canonical textual representation",
                );
            }),
        );
    }
}

/// Return whether an inherent method has the ad hoc display shape.
fn is_display_candidate(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    // Trait methods already have an explicit contract, so only inherent methods count.
    if !matches!(item.kind, ImplItemKind::Fn(..))
        || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        || !display_name(item.ident.name.as_str())
    {
        return false;
    }

    // Resolve the signature so aliases of `String` count as `String`.
    let def_id = item.owner_id.def_id;
    let sig = cx
        .tcx
        .fn_sig(def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .skip_binder();
    if !returns_string(cx, sig.output())
        || !cx.tcx.associated_item(def_id).is_method()
        || !matches!(sig.inputs(), [receiver] if matches!(receiver.kind(), ty::Ref(_, _, Mutability::Not)))
    {
        return false;
    }

    // A type that already implements `Display` offers the standard representation.
    let self_ty = cx
        .tcx
        .type_of(cx.tcx.local_parent(def_id))
        .instantiate_identity()
        .skip_norm_wip();
    !has_display_impl(cx, self_ty)
}

/// Return whether a signature returns the language `String` type.
fn returns_string(cx: &LateContext<'_>, output: ty::Ty<'_>) -> bool {
    let Some(string_def_id) = cx.tcx.lang_items().string() else {
        return false;
    };
    matches!(
        output.kind(),
        ty::Adt(adt, _) if adt.did() == string_def_id
    )
}

/// Return whether the receiver already has a usable `Display` implementation.
fn has_display_impl<'tcx>(cx: &LateContext<'tcx>, self_ty: ty::Ty<'tcx>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::Display)
        .is_none_or(|display| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(display, [self_ty], cx.param_env)
                .may_apply()
        })
}

/// Return whether the method name commonly denotes display formatting.
fn display_name(name: &str) -> bool {
    matches!(name, "to_string" | "display" | "format" | "render")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
