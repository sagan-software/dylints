#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc display formatting methods.
//!
//! It finds inherent methods that return owned strings through common display-like
//! names while excluding real `Display` implementations and methods with arguments.
//! The diagnostic points at the method declaration and recommends the standard trait
//! when the returned text is the type's canonical representation.

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
    pub AD_HOC_DISPLAY,
    Warn,
    "ad hoc display method could be `Display`",
    AdHocDisplay
}

impl<'tcx> LateLintPass<'tcx> for AdHocDisplay {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let ImplItemKind::Fn(_, _) = item.kind else {
            return;
        };

        // This lint only targets inherent ad hoc methods; trait impl methods already have an
        // explicit contract, and `Display` is the contract we want callers to use.
        if standard_display_impl(cx, item)
            || !matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
        {
            return;
        }

        let name = item.ident.name.to_ident_string();
        // Resolve the signature before checking receiver, arguments, and return type.
        let sig = cx
            .tcx
            .fn_sig(item.owner_id.def_id)
            .instantiate_identity()
            .skip_binder();

        if display_name(&name)
            && no_argument_shared_ref_method(sig.inputs())
            && string_return(cx, sig.output())
        {
            emit_span_lint_with_help(
                cx,
                AD_HOC_DISPLAY,
                item.span,
                format!("method `{name}` looks like ad hoc display formatting"),
                "implement `std::fmt::Display` when this is the canonical textual representation",
            );
        }
    }
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

/// Return whether this is the standard display impl shape.
fn standard_display_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    if !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Resolve the implemented trait by DefId so fully-qualified or renamed paths are treated the
    // same as a direct `std::fmt::Display` impl.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    display_trait(cx, trait_def_id)
}

/// Helper for display trait analysis.
fn display_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(trait_def_id).as_str(),
        "core::fmt::Display" | "std::fmt::Display"
    )
}

/// Return the display name.
fn display_name(name: &str) -> bool {
    matches!(name, "to_string" | "display" | "format" | "render")
}

/// Helper for no argument shared ref method analysis.
fn no_argument_shared_ref_method(inputs: &[Ty<'_>]) -> bool {
    matches!(inputs, [receiver] if shared_ref(*receiver))
}

/// Helper for shared ref analysis.
fn shared_ref(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(_, _, Mutability::Not))
}

/// Helper for string return analysis.
fn string_return(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    // `String` is defined in `alloc`; aliases and `std::string::String` paths lower to that ADT.
    cx.tcx.item_name(adt.did()) == sym::String && cx.tcx.crate_name(adt.did().krate) == sym::alloc
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
