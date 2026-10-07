#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for ad hoc default constructors.
//!
//! It resolves inherent zero-argument constructors whose name and return type
//! suggest a default value, then skips types that already implement `Default`.
//! The diagnostic points at the constructor and asks for a `Default`
//! implementation, which matches Clippy's `new_without_default` advice.

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
use rustc_span::sym;
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_DEFAULT,
    Warn,
    "ad hoc default constructor could be `Default`",
    AdHocDefault
}

impl<'tcx> LateLintPass<'tcx> for AdHocDefault {
    /// Check one inherent associated function for the zero-argument constructor shape.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Require an inherent function with a default-like name before resolving types.
        if !matches!(item.kind, ImplItemKind::Fn(..))
            || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            || !default_constructor_name(item.ident.name.as_str())
        {
            return;
        }

        // Compare rustc types so aliases, `Self`, and qualified paths are the same type.
        let def_id = item.owner_id.def_id;
        let self_ty = cx
            .tcx
            .type_of(cx.tcx.local_parent(def_id))
            .instantiate_identity()
            .skip_norm_wip();
        let sig = cx
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .skip_binder();
        if !sig.inputs().is_empty() {
            return;
        }
        if sig.output() != self_ty {
            return;
        }

        // A type that already implements `Default` follows the `new` plus `Default` convention.
        if has_default_impl(cx, self_ty) {
            return;
        }

        let name = item.ident.name;
        cx.emit_span_lint(
            AD_HOC_DEFAULT,
            item.span,
            DiagDecorator(move |diag| {
                let _ = diag.primary_message(format!("constructor `{name}` looks like a default value"));
                let _ = diag.help(format!(
                    "implement `Default` with this value; `{name}` can stay and return `Self::default()`"
                ));
            }),
        );
    }
}

/// Return whether the receiver type already implements `Default`.
fn has_default_impl<'tcx>(cx: &LateContext<'tcx>, self_ty: Ty<'tcx>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::Default)
        .is_none_or(|default| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(default, [self_ty], cx.param_env)
                .may_apply()
        })
}

/// Return whether the name commonly denotes a zero-argument default constructor.
fn default_constructor_name(name: &str) -> bool {
    matches!(name, "new" | "empty" | "blank" | "default_config")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
