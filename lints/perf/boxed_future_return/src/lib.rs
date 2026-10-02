#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc type variants"
)]

//! A lint to check for boxed futures in API return types.
//!
//! It follows returned types through standard boxes, references, tuples, and
//! nested generic arguments to find a dynamic `Future`. The diagnostic focuses
//! on explicit return syntax and suggests async APIs when the box is avoidable.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{FnDecl, FnRetTy, TraitFn, TraitItem, TraitItemKind, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, GenericArgKind, Ty};
use rustc_span::{Span, def_id::LocalDefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BOXED_FUTURE_RETURN,
    Warn,
    "function returns a boxed future",
    BoxedFutureReturn
}

impl<'tcx> LateLintPass<'tcx> for BoxedFutureReturn {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        _body: &'tcx rustc_hir::Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }

        check_return_ty(cx, local_def_id, explicit_return_span(decl));
    }

    /// Check trait item for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        if let TraitItemKind::Fn(sig, TraitFn::Required(_)) = item.kind {
            check_return_ty(cx, item.owner_id.def_id, explicit_return_span(sig.decl));
        }
    }
}

/// Return the span for explicit return.
const fn explicit_return_span(decl: &FnDecl<'_>) -> Option<Span> {
    let FnRetTy::Return(output) = decl.output else {
        return None;
    };

    Some(output.span)
}

/// Check return ty for this lint.
fn check_return_ty(cx: &LateContext<'_>, local_def_id: LocalDefId, span: Option<Span>) {
    // Inspect only explicit return types with a source span for diagnostics.
    let Some(span) = span else {
        return;
    };

    let output = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder();

    if boxed_future_ty(cx, output) {
        emit_span_lint_with_help(
            cx,
            BOXED_FUTURE_RETURN,
            span,
            "this API returns a boxed future",
            "prefer `async fn`, native async trait methods, or `async-trait` unless boxing is required",
        );
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
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

/// Return type information for boxed future.
fn boxed_future_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    match ty.kind() {
        ty::Adt(adt, args) if is_std_box(cx, adt.did()) => dyn_future_ty(cx, args.type_at(0)),
        ty::Adt(_, args) => args.iter().any(
            |arg| matches!(arg.kind(), GenericArgKind::Type(inner) if boxed_future_ty(cx, inner)),
        ),
        ty::Ref(_, inner, _) => boxed_future_ty(cx, *inner),
        ty::Tuple(types) => types.iter().any(|inner| boxed_future_ty(cx, inner)),
        _ => false,
    }
}

/// Return whether std box.
fn is_std_box(cx: &LateContext<'_>, def_id: rustc_span::def_id::DefId) -> bool {
    cx.tcx.item_name(def_id).as_str() == "Box"
        && matches!(cx.tcx.crate_name(def_id.krate).as_str(), "alloc" | "std")
}

/// Return type information for dyn future.
fn dyn_future_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Dynamic(predicates, _) = ty.kind() else {
        return false;
    };

    predicates.principal_def_id().is_some_and(|def_id| {
        cx.tcx.def_path_str(def_id) == "core::future::future::Future"
            || cx.tcx.def_path_str(def_id) == "std::future::Future"
    })
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
