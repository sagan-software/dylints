#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only collection return types are relevant and diagnostics are configured in place"
)]

//! A lint to check for eager collection return surfaces.
//!
//! It examines public source-authored functions whose declared return type is a
//! standard collection and whose tail expression calls the resolved
//! `Iterator::collect` method. The diagnostic describes a lazy iterator return
//! as an option while preserving APIs whose ownership contract requires eager
//! materialization.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{Body, Expr, ExprKind, FnDecl, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub COLLECT_RETURN,
    Warn,
    "tail `.collect()` eagerly fixes an API to return a collection",
    CollectReturn
}

impl<'tcx> LateLintPass<'tcx> for CollectReturn {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Skip closures, generated functions, functions outside the exported API, and trait
        // impl methods, whose return type the trait fixes.
        let is_candidate = !matches!(kind, FnKind::Closure)
            && !span.from_expansion()
            && cx.effective_visibilities.is_exported(local_def_id)
            && cx
                .tcx
                .trait_impl_of_assoc(local_def_id.to_def_id())
                .is_none();

        // Require a supported collection return type and a tail collect call.
        if is_candidate
            && let Some(collection) = collection_return(cx, local_def_id)
            && let Some(collect_span) = tail_collect_call(cx, body.value)
        {
            // Point at the eager collect call while preserving ownership as a valid exception.
            let collection_label = collection.label();
            emit_span_lint_with_help(
                cx,
                COLLECT_RETURN,
                collect_span,
                format!(
                    "this `{collection_label}` return value comes from a tail `.collect()` call"
                ),
                "consider returning `impl Iterator<Item = ...>` when callers can consume lazily; keep the collection when ownership or materialization is part of the contract",
            );
        }
    }
}

/// Standard collection families recognized by the public-return analysis.
#[derive(Clone, Copy)]
enum CollectionKind {
    /// Boxed slice materialization.
    BoxSlice,
    /// Vector materialization.
    Vec,
    /// Hash map materialization.
    HashMap,
    /// Ordered map materialization.
    BTreeMap,
    /// Hash set materialization.
    HashSet,
    /// Ordered set materialization.
    BTreeSet,
}

impl CollectionKind {
    /// Returns the stable label used in the diagnostic.
    const fn label(self) -> &'static str {
        match self {
            Self::BoxSlice => "Box<[T]>",
            Self::Vec => "Vec",
            Self::HashMap => "HashMap",
            Self::BTreeMap => "BTreeMap",
            Self::HashSet => "HashSet",
            Self::BTreeSet => "BTreeSet",
        }
    }
}

/// Helper for collection return analysis.
fn collection_return(cx: &LateContext<'_>, local_def_id: LocalDefId) -> Option<CollectionKind> {
    let output = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder();

    collection_ty(cx, output)
}

/// Return the standard collection family of a resolved return type.
fn collection_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<CollectionKind> {
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    // Recognize boxed slices before the named collection ADTs.
    if adt.is_box() {
        return args
            .types()
            .next()
            .is_some_and(|inner| matches!(inner.kind(), ty::Slice(_)))
            .then_some(CollectionKind::BoxSlice);
    }

    // Match the standard collections by diagnostic item so aliases and re-exports resolve.
    COLLECTION_ITEMS
        .iter()
        .find(|(item, _)| cx.tcx.is_diagnostic_item(Symbol::intern(item), adt.did()))
        .map(|(_, kind)| *kind)
}

/// Diagnostic items of the standard collections mapped to their families.
const COLLECTION_ITEMS: [(&str, CollectionKind); 5] = [
    ("Vec", CollectionKind::Vec),
    ("HashMap", CollectionKind::HashMap),
    ("BTreeMap", CollectionKind::BTreeMap),
    ("HashSet", CollectionKind::HashSet),
    ("BTreeSet", CollectionKind::BTreeSet),
];

/// Helper for tail collect call analysis.
fn tail_collect_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    let ExprKind::Block(block, _) = expr.kind else {
        return collect_call_span(cx, expr);
    };

    // Only the semicolon-free final expression is the returned value.
    let tail = block.expr?;
    collect_call_span(cx, tail)
}

/// Collect call span used by the lint.
fn collect_call_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    match expr.kind {
        ExprKind::MethodCall(segment, _receiver, _args, _call_span) => Some((segment, expr.hir_id)),
        _ => None,
    }
    .and_then(|(segment, hir_id)| {
        cx.typeck_results()
            .type_dependent_def_id(hir_id)
            .map(|def_id| (segment, def_id))
    })
    .and_then(|(segment, def_id)| {
        cx.tcx
            .opt_associated_item(def_id)
            .and_then(|assoc_item| assoc_item.trait_item_or_self().ok())
            .and_then(|trait_item| cx.tcx.trait_of_assoc(trait_item))
            .map(|trait_def_id| (segment, def_id, trait_def_id))
    })
    // Require the resolved `Iterator::collect` item so local `collect` methods stay excluded.
    .filter(|(_, def_id, _)| cx.tcx.is_diagnostic_item(sym::iterator_collect_fn, *def_id))
    .map(|(segment, _, _)| segment.ident.span)
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

    // Use rustc's native diagnostic decorator to avoid depending on Clippy utilities.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
