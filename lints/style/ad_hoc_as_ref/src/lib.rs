#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc AsRef-style methods.
//!
//! It inspects source-authored inherent methods whose names and signatures
//! expose a reference conversion without implementing the standard `AsRef`
//! trait. Resolved receiver and return types keep unrelated accessors,
//! consuming conversions, and methods with additional work outside the lint's
//! recommendation.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FnDecl, ImplItem, ImplItemImplKind, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_AS_REF,
    Warn,
    "ad hoc borrowed accessor could be `AsRef`",
    AdHocAsRef
}

impl<'tcx> LateLintPass<'tcx> for AdHocAsRef {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };
        let name = item.ident.name.to_ident_string();

        // Leave canonical AsRef implementations to the trait contract.
        if standard_as_ref_method(cx, item, &name) {
            return;
        }

        check_candidate(cx, &name, sig.decl, item.span, item.owner_id.def_id);
    }
}

/// Check candidate for this lint.
fn check_candidate(
    cx: &LateContext<'_>,
    name: &str,
    decl: &FnDecl<'_>,
    span: Span,
    local_def_id: LocalDefId,
) {
    // Evaluate receiver, vocabulary, and resolved return-shape requirements independently.
    let has_self_receiver = decl.implicit_self().has_implicit_self();
    let has_accessor_name = accessor_name(name);
    let returns_receiver_reference = reference_receiver_and_return(cx, local_def_id);
    // Require the complete borrowed-accessor shape before diagnosing.
    if !has_self_receiver || !has_accessor_name || !returns_receiver_reference {
        return;
    }

    // Recommend the standard trait for the detected canonical borrowed view.
    emit_span_lint_with_help(
        cx,
        AD_HOC_AS_REF,
        span,
        format!("method `{name}` looks like an `AsRef` accessor"),
        "implement `AsRef<T>` when this exposes the canonical borrowed view",
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

/// Return the accessor name.
fn accessor_name(name: &str) -> bool {
    (name.starts_with("as_") || name.starts_with("get_")) && !name.starts_with("get_or_")
}

/// Helper for reference receiver and return analysis.
fn reference_receiver_and_return(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let fn_sig = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let inputs = fn_sig.inputs().skip_binder();
    if inputs.len() != 1 {
        return false;
    }

    // Use the lowered signature so aliases like `type PathRef<'a> = &'a Path` are recognized as
    // borrowed returns instead of being treated as source-level path syntax.
    inputs
        .first()
        .is_some_and(|input| reference_ty(*input) && reference_ty(fn_sig.output().skip_binder()))
}

/// Return type information for reference.
fn reference_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Ref(..))
}

/// Return whether this is the standard as ref method shape.
fn standard_as_ref_method(cx: &LateContext<'_>, item: &ImplItem<'_>, name: &str) -> bool {
    if name != "as_ref" || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Query the parent impl so renamed imports and fully-qualified trait paths resolve by trait
    // identity rather than by the spelling used in the impl header.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    as_ref_trait(cx, trait_def_id)
}

/// Helper for as ref trait analysis.
fn as_ref_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::AsRef, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::convert::AsRef" | "std::convert::AsRef"
        )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
