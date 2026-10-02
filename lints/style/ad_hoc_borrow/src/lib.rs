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
//! outside the recommendation, so a suggested trait migration preserves the
//! method's ownership boundary.

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
    pub AD_HOC_BORROW,
    Warn,
    "ad hoc borrowed accessor could be `Borrow`",
    AdHocBorrow
}

impl<'tcx> LateLintPass<'tcx> for AdHocBorrow {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(_, _) = item.kind else {
            return;
        };

        // Leave canonical Borrow implementations to the trait contract.
        if standard_borrow_impl(cx, item) {
            return;
        }

        // Resolve the method signature before checking its borrowed accessor shape.
        let name = item.ident.name.to_ident_string();
        let sig = cx
            .tcx
            .fn_sig(item.owner_id.def_id)
            .instantiate_identity()
            .skip_binder();

        if borrow_name(&name)
            && no_argument_shared_ref_method(sig.inputs())
            && shared_ref(sig.output())
        {
            emit_span_lint_with_help(
                cx,
                AD_HOC_BORROW,
                item.span,
                format!("method `{name}` looks like a `Borrow` accessor"),
                "implement `std::borrow::Borrow<T>` only when equality and hashing match the borrowed view",
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

/// Return whether this is the standard borrow impl shape.
fn standard_borrow_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    if !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Resolve the implemented trait by DefId so renamed imports and fully-qualified paths are
    // treated the same as `std::borrow::Borrow`.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    borrow_trait(cx, trait_def_id)
}

/// Helper for borrow trait analysis.
fn borrow_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::Borrow, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::borrow::Borrow" | "std::borrow::Borrow"
        )
}

/// Return the borrow name.
fn borrow_name(name: &str) -> bool {
    name.starts_with("borrow_")
}

/// Helper for no argument shared ref method analysis.
fn no_argument_shared_ref_method(inputs: &[Ty<'_>]) -> bool {
    matches!(inputs, [receiver] if shared_ref(*receiver))
}

/// Helper for shared ref analysis.
fn shared_ref(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(_, _, Mutability::Not))
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
