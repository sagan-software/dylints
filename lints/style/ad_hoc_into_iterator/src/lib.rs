#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for ad hoc IntoIterator-style methods.
//!
//! It inspects source-authored implementation methods whose signatures and
//! bodies expose iteration through a custom method rather than the standard
//! `IntoIterator` contract. Resolved return types and receiver behavior keep
//! unrelated builder, conversion, and side-effect methods out of scope.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{ImplItem, ImplItemImplKind, ImplItemKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty, Unnormalized};
use rustc_span::{Span, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub AD_HOC_INTO_ITERATOR,
    Warn,
    "ad hoc iteration method could be `IntoIterator`",
    AdHocIntoIterator
}

impl<'tcx> LateLintPass<'tcx> for AdHocIntoIterator {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        // Restrict the analysis to functions declared inside an implementation.
        let ImplItemKind::Fn(_, _) = item.kind else {
            return;
        };

        // Leave canonical `IntoIterator` implementations to the trait contract.
        let name = item.ident.name.to_ident_string();
        if standard_into_iterator_method(cx, item, &name) {
            return;
        }

        // Resolve the method signature so aliases do not affect its iteration shape.
        let sig = cx
            .tcx
            .fn_sig(item.owner_id.def_id)
            .instantiate_identity()
            .skip_binder();

        if iteration_name(&name)
            && receiver_only_method(sig.inputs())
            && iterable_return(cx, sig.output())
        {
            emit_span_lint_with_help(
                cx,
                AD_HOC_INTO_ITERATOR,
                item.span,
                format!("method `{name}` looks like ad hoc iteration"),
                "implement `IntoIterator` when this is the canonical iteration view",
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

/// Return the iteration name.
fn iteration_name(name: &str) -> bool {
    (name.starts_with("into_") || name.starts_with("iter_") || name == "items")
        && !domain_policy_name(name)
}

/// Return the domain policy name.
fn domain_policy_name(name: &str) -> bool {
    [
        "active",
        "allocated",
        "batched",
        "chunk",
        "chunked",
        "cloned",
        "dedup",
        "deduplicated",
        "enabled",
        "filter",
        "filtered",
        "matching",
        "owned",
        "page",
        "paged",
        "sort",
        "sorted",
        "unique",
        "valid",
        "visible",
    ]
    .iter()
    .any(|word| name.contains(word))
}

/// Helper for receiver only method analysis.
const fn receiver_only_method(inputs: &[Ty<'_>]) -> bool {
    inputs.len() == 1
}

/// Helper for iterable return analysis.
fn iterable_return(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    std_vec_ty(cx, ty) || std_iterator_ty(cx, ty) || opaque_iterator_ty(cx, ty)
}

/// Return type information for std vec.
fn std_vec_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    cx.tcx.item_name(adt.did()).as_str() == "Vec"
        && matches!(cx.tcx.crate_name(adt.did().krate).as_str(), "alloc" | "std")
}

/// Return type information for std iterator.
fn std_iterator_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    matches!(
        cx.tcx.def_path_str(adt.did()).as_str(),
        "alloc::vec::into_iter::IntoIter" | "std::vec::IntoIter"
    )
}

/// Return type information for opaque iterator.
fn opaque_iterator_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Alias(_, alias) = ty.kind() else {
        return false;
    };
    let ty::AliasTyKind::Opaque { def_id } = alias.kind else {
        return false;
    };

    // Read rustc's lowered opaque bounds so `impl std::iter::Iterator` and imported
    // `Iterator` spellings share the same trait identity check.
    cx.tcx
        .explicit_item_bounds(def_id)
        .iter_identity_copied()
        .map(Unnormalized::skip_norm_wip)
        .any(|(predicate, _)| {
            predicate.as_trait_clause().is_some_and(|clause| {
                let trait_def_id = clause.skip_binder().trait_ref.def_id;
                iterator_trait(cx, trait_def_id)
            })
        })
}

/// Return whether this is the standard into iterator method shape.
fn standard_into_iterator_method(cx: &LateContext<'_>, item: &ImplItem<'_>, name: &str) -> bool {
    if name != "into_iter" || !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) {
        return false;
    }

    // Query the parent impl so aliases and fully-qualified trait paths are resolved by rustc
    // instead of by the spelling used in the impl header.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    into_iterator_trait(cx, trait_def_id)
}

/// Helper for into iterator trait analysis.
fn into_iterator_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::IntoIterator, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::iter::traits::collect::IntoIterator" | "std::iter::IntoIterator"
        )
}

/// Helper for iterator trait analysis.
fn iterator_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    if cx.tcx.is_diagnostic_item(sym::Iterator, trait_def_id) {
        return true;
    }

    matches!(
        cx.tcx.def_path_str(trait_def_id).as_str(),
        "core::iter::traits::iterator::Iterator" | "std::iter::Iterator"
    )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
