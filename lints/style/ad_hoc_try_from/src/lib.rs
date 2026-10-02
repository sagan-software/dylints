#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]
#![warn(unused_extern_crates)]

//! A lint to check for ad hoc fallible conversion functions.
//!
//! It checks free functions and inherent associated functions whose name and
//! resolved one-argument signature describe a conversion that a `TryFrom`
//! implementation could express. Resolved types decide whether that
//! implementation is legal and new: one side must be local, the target must be
//! a real value, and no `TryFrom` implementation may already apply.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::DiagDecorator;
use rustc_hir::{Body, FnDecl, ImplItem, ImplItemImplKind, ImplItemKind, intravisit::FnKind};
use rustc_infer::infer::TyCtxtInferExt;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, def_id::LocalDefId, sym};
use rustc_trait_selection::infer::InferCtxtExt;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_TRY_FROM,
    Warn,
    "ad hoc fallible conversion function could be `TryFrom`",
    AdHocTryFrom
}

impl<'tcx> LateLintPass<'tcx> for AdHocTryFrom {
    /// Check one free function.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        _body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if let FnKind::ItemFn(ident, ..) = kind {
            check_candidate(cx, ident.name, local_def_id, span);
        }
    }

    /// Check one inherent associated function; trait items have names fixed by their trait.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // `TryFrom::try_from` is an associated conversion, so receiver methods stay out of scope.
        let def_id = item.owner_id.def_id;
        if matches!(item.kind, ImplItemKind::Fn(..))
            && matches!(item.impl_kind, ImplItemImplKind::Inherent { .. })
            && !cx.tcx.associated_item(def_id).is_method()
        {
            check_candidate(cx, item.ident.name, def_id, item.span);
        }
    }
}

/// Report one function whose name and signature fit a new `TryFrom` implementation.
fn check_candidate(cx: &LateContext<'_>, name: Symbol, local_def_id: LocalDefId, span: Span) {
    // Resolve the named signature before applying conversion coherence rules.
    let Some((input, target)) = conversion_signature(cx, name, local_def_id) else {
        return;
    };

    // Keep only conversions with a local endpoint and no existing standard implementation.
    if !is_valid_try_from_conversion(cx, input, target) {
        return;
    }

    // Emit the conversion guidance after all semantic restrictions pass.
    cx.emit_span_lint(
        AD_HOC_TRY_FROM,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(format!(
                "function `{name}` looks like a fallible conversion"
            ));
            let _ = diag.help(format!(
                "implement `TryFrom<{input}> for {target}` when the conversion has one canonical meaning"
            ));
        }),
    );
}

/// Return the input and target types of a fallible conversion signature.
fn conversion_signature<'tcx>(
    cx: &LateContext<'tcx>,
    name: Symbol,
    local_def_id: LocalDefId,
) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    // Reject names outside the fallible-conversion vocabulary before resolving types.
    if !is_conversion_name(name.as_str()) {
        return None;
    }
    // Erase late-bound lifetimes so the trait solver sees closed types.
    let fn_sig = cx.tcx.instantiate_bound_regions_with_erased(
        cx.tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip(),
    );
    let ([input], ty::Adt(result, args)) = (fn_sig.inputs(), fn_sig.output().kind()) else {
        return None;
    };
    // Resolve only standard `Result` outputs with one target type.
    if !cx.tcx.is_diagnostic_item(sym::Result, result.did()) {
        return None;
    }
    Some((*input, args.types().next()?))
}

/// Return whether a fallible conversion satisfies shape, locality, and coherence rules.
fn is_valid_try_from_conversion<'tcx>(
    cx: &LateContext<'tcx>,
    input: Ty<'tcx>,
    target: Ty<'tcx>,
) -> bool {
    !matches!(input.kind(), ty::Param(_))
        && !matches!(target.kind(), ty::Never)
        && !matches!(target.kind(), ty::Tuple(fields) if fields.is_empty())
        && input != target
        && has_local_endpoint(input, target)
        && !has_try_from_impl(cx, target, input)
}

/// Return whether either conversion endpoint is a local algebraic data type.
fn has_local_endpoint(input: Ty<'_>, target: Ty<'_>) -> bool {
    is_local_outer_adt(input) || is_local_outer_adt(target)
}

/// Return whether the name uses fallible-conversion vocabulary.
fn is_conversion_name(name: &str) -> bool {
    ["make_", "build_", "convert_", "map_", "try_", "validate_"]
        .iter()
        .any(|prefix| name.starts_with(prefix))
}

/// Return whether the outer semantic type is an ADT defined in this crate.
fn is_local_outer_adt(mut ty: Ty<'_>) -> bool {
    // Peel references because `&T` is fundamental for the orphan rule.
    while let ty::Ref(_, inner, _) = ty.kind() {
        ty = *inner;
    }

    matches!(ty.kind(), ty::Adt(adt, _) if adt.did().is_local())
}

/// Return whether `target: TryFrom<source>` may already hold in the item's environment.
///
/// This includes the standard blanket implementation for every `Into` conversion.
fn has_try_from_impl<'tcx>(cx: &LateContext<'tcx>, target: Ty<'tcx>, source: Ty<'tcx>) -> bool {
    cx.tcx
        .get_diagnostic_item(sym::TryFrom)
        .is_none_or(|try_from| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(try_from, [target, source], cx.param_env)
                .may_apply()
        })
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
