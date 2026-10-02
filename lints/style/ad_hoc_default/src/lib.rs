#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for ad hoc default constructors.
//!
//! It resolves inherent zero-argument constructors whose name and return type
//! suggest a default value, then excludes constructors with inputs, receivers,
//! or custom behavior. The diagnostic points at the method and recommends the
//! standard `Default` trait when its contract can express the same value.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FnDecl, ImplItem, ImplItemImplKind, ImplItemKind, ImplicitSelfKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, def_id::DefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_DEFAULT,
    Warn,
    "ad hoc default constructor could be `Default`",
    AdHocDefault
}

impl<'tcx> LateLintPass<'tcx> for AdHocDefault {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };

        let name = item.ident.name.to_ident_string();
        // Leave canonical `Default::default` implementations to the trait contract.
        if standard_default_impl(cx, item) {
            return;
        }

        check_candidate(cx, item, &name, sig.decl, item.span);
    }
}

/// Check candidate for this lint.
fn check_candidate(
    cx: &LateContext<'_>,
    item: &ImplItem<'_>,
    name: &str,
    decl: &FnDecl<'_>,
    span: Span,
) {
    // Require the complete zero-argument inherent-constructor shape before diagnosing.
    if !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        || decl.implicit_self() != ImplicitSelfKind::None
        || !decl.inputs.is_empty()
        || !default_constructor_name(name)
        || !returns_implementing_type(cx, item)
    {
        return;
    }

    // Explain the standard trait that should express the detected default value.
    emit_span_lint_with_help(
        cx,
        AD_HOC_DEFAULT,
        span,
        format!("constructor `{name}` looks like a default value"),
        "implement `Default` when this is the canonical zero-argument value",
    );
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

    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return whether this is the standard default impl shape.
fn standard_default_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    if item.ident.name.as_str() != "default"
        || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. })
    {
        return false;
    }

    // Resolve the parent impl's trait rather than trusting the written path, so
    // qualified `std::default::Default` impls and imports are handled the same.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    standard_default_trait(cx, trait_def_id)
}

/// Return whether this is the standard default trait shape.
fn standard_default_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(trait_def_id).as_str(),
        "core::default::Default" | "std::default::Default"
    )
}

/// Return whether the item returns implementing type.
fn returns_implementing_type(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    // Compare rustc types instead of the written return path so aliases,
    // `Self`, and fully-qualified type names all use the same contract.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let fn_sig = cx.tcx.fn_sig(item.owner_id.def_id).instantiate_identity();

    fn_sig.skip_binder().output() == self_ty
}

/// Return the default constructor name.
fn default_constructor_name(name: &str) -> bool {
    matches!(name, "new" | "empty" | "blank" | "default_config")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
