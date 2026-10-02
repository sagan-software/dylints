#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]
#![warn(unused_extern_crates)]

//! A lint to check for ad hoc infallible conversion functions.
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
use rustc_hir::{Body, ImplItem, ImplItemImplKind, ImplItemKind, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_FROM,
    Warn,
    "ad hoc infallible conversion function could be `From`",
    AdHocFrom
}

impl<'tcx> LateLintPass<'tcx> for AdHocFrom {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        _body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        let FnKind::ItemFn(ident, ..) = kind else {
            return;
        };

        let name = ident.name.to_ident_string();
        check_candidate(cx, &name, span, local_def_id);
    }

    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(_, _) = item.kind else {
            return;
        };

        // Leave canonical `From::from` implementations to the trait contract.
        if standard_from_impl(cx, item) {
            return;
        }

        let name = item.ident.name.to_ident_string();
        check_candidate(cx, &name, item.span, item.owner_id.def_id);
    }
}

/// Check candidate for this lint.
fn check_candidate(cx: &LateContext<'_>, name: &str, span: Span, local_def_id: LocalDefId) {
    // Require both conversion vocabulary and a compatible one-input signature.
    if !conversion_name(name) || !one_input_concrete_conversion(cx, local_def_id) {
        return;
    }

    // Recommend the standard conversion trait for the detected canonical shape.
    emit_span_lint_with_help(
        cx,
        AD_HOC_FROM,
        span,
        format!("function `{name}` looks like an infallible conversion"),
        "implement `From` when the conversion has one canonical meaning",
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

/// Return the conversion name.
fn conversion_name(name: &str) -> bool {
    [
        "make_",
        "build_",
        "convert_",
        "extract_",
        "extracted_",
        "from_",
    ]
    .iter()
    .any(|prefix| name.starts_with(prefix))
        && !name.contains("_and_")
}

/// Helper for one input concrete conversion analysis.
fn one_input_concrete_conversion(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let fn_sig = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_binder();
    let [input] = fn_sig.inputs() else {
        return false;
    };
    let output = fn_sig.output();

    // Compare lowered types so aliases and qualified paths do not affect the decision.
    concrete_target_type(cx, output)
        && meaningful_input(*input)
        && *input != output
        && from_impl_is_legal(*input, output)
}

/// Return whether either side gives a local type for an orphan-rule-compliant impl.
fn from_impl_is_legal(input: Ty<'_>, output: Ty<'_>) -> bool {
    outer_local_type(input) || outer_local_type(output)
}

/// Return whether the outer type, after borrowing, is defined in this crate.
fn outer_local_type(mut ty: Ty<'_>) -> bool {
    // Peel references because the orphan-rule decision belongs to the referenced type.
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }

    matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local())
}

/// Return type information for concrete target.
fn concrete_target_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    !cx.tcx.is_diagnostic_item(sym::Result, adt.did())
        && !cx.tcx.is_diagnostic_item(sym::Option, adt.did())
}

/// Helper for meaningful input analysis.
fn meaningful_input(ty: Ty<'_>) -> bool {
    !matches!(ty.kind(), ty::Never) && !matches!(ty.kind(), ty::Tuple(fields) if fields.is_empty())
}

/// Return whether this is the standard from impl shape.
fn standard_from_impl(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    if item.ident.name.as_str() != "from"
        || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. })
    {
        return false;
    }

    // Resolve the parent impl trait by DefId so qualified and imported `From`
    // impls are skipped without trusting source spelling.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    from_trait(cx, trait_def_id)
}

/// Helper for from trait analysis.
fn from_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::From, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::convert::From" | "std::convert::From"
        )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
