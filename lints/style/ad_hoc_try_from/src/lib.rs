#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]
#![warn(unused_extern_crates)]

//! A lint to check for ad hoc fallible conversion functions.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, FnDecl, ImplItem, ImplItemImplKind, ImplItemKind, ImplicitSelfKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_TRY_FROM,
    Warn,
    "ad hoc fallible conversion function could be `TryFrom`",
    AdHocTryFrom
}

impl<'tcx> LateLintPass<'tcx> for AdHocTryFrom {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        _body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        let FnKind::ItemFn(ident, ..) = kind else {
            return;
        };

        let name = ident.name.to_ident_string();
        check_candidate(cx, &name, decl, local_def_id, span);
    }

    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(sig, _) = item.kind else {
            return;
        };
        let name = item.ident.name.to_ident_string();

        // Leave canonical `TryFrom::try_from` implementations to the trait contract.
        if name == "try_from" && standard_try_from_impl(cx, item) {
            return;
        }

        check_candidate(cx, &name, sig.decl, item.owner_id.def_id, item.span);
    }
}

/// Check candidate for this lint.
fn check_candidate(
    cx: &LateContext<'_>,
    name: &str,
    decl: &FnDecl<'_>,
    local_def_id: LocalDefId,
    span: Span,
) {
    // Resolve the signature so aliases do not affect conversion shape checks.
    let fn_sig = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    let inputs = fn_sig.inputs().skip_binder();
    let [input] = inputs else {
        return;
    };

    // Combine API vocabulary, return shape, and orphan-rule legality.
    if !single_conversion_input(decl)
        || !conversion_name(name)
        || !result_return(cx, fn_sig.output().skip_binder())
        || !is_legal_try_from_shape(cx, *input, fn_sig.output().skip_binder())
    {
        return;
    }

    // Recommend the standard fallible conversion trait for the canonical shape.
    emit_span_lint_with_help(
        cx,
        AD_HOC_TRY_FROM,
        span,
        format!("function `{name}` looks like a fallible conversion"),
        "implement `TryFrom` when the conversion has one canonical meaning",
    );
}

/// Helper for single conversion input analysis.
fn single_conversion_input(decl: &FnDecl<'_>) -> bool {
    // `TryFrom::try_from` is an associated conversion, so receiver methods stay out of scope even
    // when they take one explicit argument and return a `Result`.
    decl.implicit_self() == ImplicitSelfKind::None
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

/// Return whether this is the standard try from impl shape.
fn standard_try_from_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    if !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Query the parent impl so aliases and fully-qualified trait paths are
    // resolved by rustc instead of guessed from source spelling.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    try_from_trait(cx, trait_def_id)
}

/// Helper for try from trait analysis.
fn try_from_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::TryFrom, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::convert::TryFrom" | "std::convert::TryFrom"
        )
}

/// Return the conversion name.
fn conversion_name(name: &str) -> bool {
    ["make_", "build_", "convert_", "map_", "try_", "validate_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Helper for result return analysis.
fn result_return(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, args) = ty.kind() else {
        return false;
    };

    // Compare the resolved ADT against rustc's `Result` diagnostic item so local `Result`
    // lookalikes and path aliases do not matter.
    cx.tcx.is_diagnostic_item(sym::Result, adt.did()) && args.len() == 2
}

/// Return whether Rust's coherence rules permit the suggested `TryFrom` impl.
fn is_legal_try_from_shape(cx: &LateContext<'_>, input: Ty<'_>, output: Ty<'_>) -> bool {
    let ty::Adt(result, args) = output.kind() else {
        return false;
    };
    if !cx.tcx.is_diagnostic_item(sym::Result, result.did()) {
        return false;
    }

    // A foreign trait impl needs a local input or a local outer target type.
    is_local_outer_adt(input) || is_local_outer_adt(args.type_at(0))
}

/// Return whether the outer semantic type is an ADT defined in this crate.
fn is_local_outer_adt(mut ty: Ty<'_>) -> bool {
    // Peel references because the orphan-rule decision belongs to the referenced type.
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }

    matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local())
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
