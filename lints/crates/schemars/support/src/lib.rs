#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Schemars-specific private lints.
//!
//! The helpers resolve Schemars definitions through rustc metadata and inspect
//! attributes in source order. They keep each lint focused on one public API
//! contract while avoiding source-text guesses for type-aware checks.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::{
    Crate,
    ast::{self, AttrStyle, Attribute, ModKind},
};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{EarlyContext, LateContext, LintContext as _};
use rustc_middle::ty;
use rustc_span::def_id::{DefId, LocalDefId};

use dylint_linting as _;

/// Return Schemars attributes that exactly duplicate a sibling Serde attribute.
///
/// An attribute matches when its single key equals one of `keys` exactly, so
/// `rename` does not match `rename_all`. The returned spans identify only the
/// redundant Schemars attributes, preserving the original item order so
/// diagnostics and machine fixes remain predictable.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, krate| {
///     let _ = schemars_support::redundant_serde_attribute_spans(cx, krate, &["rename"]);
/// };
/// ```
pub fn redundant_serde_attribute_spans(
    cx: &EarlyContext<'_>,
    krate: &Crate,
    keys: &[&str],
) -> Vec<rustc_span::Span> {
    // Preserve crate item order so diagnostics follow source order.
    let mut spans = Vec::new();
    for item in &krate.items {
        collect_redundant_attrs(cx, item, keys, &mut spans);
    }
    spans
}

/// Traverse one item and its fields, variants, and loaded modules.
fn collect_redundant_attrs(
    cx: &EarlyContext<'_>,
    item: &ast::Item,
    keys: &[&str],
    spans: &mut Vec<rustc_span::Span>,
) {
    // Check attributes attached directly to the item before its children.
    collect_attr_pair(cx, &item.attrs, keys, spans);
    // Traverse only item kinds that can carry the compared attributes.
    if let ast::ItemKind::Struct(_, _, data) | ast::ItemKind::Union(_, _, data) = &item.kind {
        for field in data.fields() {
            collect_attr_pair(cx, &field.attrs, keys, spans);
        }
    }
    // Visit variant and field attributes in their declaration order.
    if let ast::ItemKind::Enum(_, _, definition) = &item.kind {
        for variant in &definition.variants {
            collect_attr_pair(cx, &variant.attrs, keys, spans);
            for field in variant.data.fields() {
                collect_attr_pair(cx, &field.attrs, keys, spans);
            }
        }
    }
    if let ast::ItemKind::Mod(_, _, ModKind::Loaded(items, ..)) = &item.kind {
        // Recurse only into modules whose contents are available in this AST.
        for child in items {
            collect_redundant_attrs(cx, child, keys, spans);
        }
    }
}

/// Compare exact single-key attribute spellings after substituting the namespace.
fn collect_attr_pair(
    cx: &EarlyContext<'_>,
    attrs: &[Attribute],
    keys: &[&str],
    spans: &mut Vec<rustc_span::Span>,
) {
    // Retain only outer attributes with recoverable source text.
    let snippets: Vec<_> = attrs
        .iter()
        .filter(|attr| matches!(attr.style, AttrStyle::Outer))
        .filter_map(|attr| {
            cx.sess()
                .source_map()
                .span_to_snippet(attr.span)
                .ok()
                .map(|snippet| (attr, snippet))
        })
        .collect();
    // Compare each Schemars spelling with its exact Serde namespace substitution.
    for (attr, snippet) in &snippets {
        // Require an exact key so `rename` cannot match `rename_all`.
        let Some(suffix) = keys.iter().find_map(|key| {
            snippet
                .strip_prefix("#[schemars(")
                .and_then(|rest| rest.strip_prefix(key))
                .filter(|rest| {
                    !rest.starts_with(|next: char| next == '_' || next.is_alphanumeric())
                })
        }) else {
            continue;
        };
        if suffix.contains(',') {
            continue;
        }
        // Avoid treating a multi-key attribute as an exact duplicate.
        let serde = snippet.replacen("#[schemars(", "#[serde(", 1);
        // Report the Schemars span only when the sibling spelling is present.
        if snippets.iter().any(|(_, candidate)| candidate == &serde) {
            spans.push(attr.span);
        }
    }
}

/// Declare one redundant Schemars-over-Serde attribute lint.
#[macro_export]
macro_rules! declare_redundant_serde_attribute_lint {
    ($lint:ident, $pass:ident, [$($key:literal),+ $(,)?], $description:literal) => {
        dylint_support::documented_early_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl rustc_lint::EarlyLintPass for $pass {
            /// Check item, variant, and field attributes in the crate AST.
            fn check_crate(&mut self, cx: &rustc_lint::EarlyContext<'_>, krate: &rustc_ast::Crate) {
                for span in $crate::redundant_serde_attribute_spans(cx, krate, &[$($key),+]) {
                    cx.emit_span_lint(
                        $lint,
                        span,
                        rustc_errors::DiagDecorator(|diagnostic| {
                            let _configured_diagnostic = diagnostic
                                .primary_message("this Schemars attribute duplicates Serde")
                                .span_suggestion(
                                    span,
                                    "remove the redundant Schemars attribute",
                                    String::new(),
                                    rustc_errors::Applicability::MachineApplicable,
                                );
                        }),
                    );
                }
            }
        }

        /// Run the UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return whether a trait definition is Schemars's `JsonSchema`.
///
/// Resolution is crate-aware, so a local trait with the same name cannot produce
/// a false positive for a lint that targets the external Schemars contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, def_id| {
///     let _ = schemars_support::is_json_schema_trait(cx, def_id);
/// };
/// ```
pub fn is_json_schema_trait(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
        && cx.tcx.item_name(def_id).as_str() == "JsonSchema"
}

/// Return whether an expression calls one exact Schemars method.
///
/// The method name and defining crate must both match, which excludes unrelated
/// extension traits and local methods that happen to use the same spelling.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_name| {
///     let _ = schemars_support::is_schemars_method_call(cx, expr, expected_name);
/// };
/// ```
pub fn is_schemars_method_call(cx: &LateContext<'_>, expr: &Expr<'_>, expected_name: &str) -> bool {
    // Reject non-method expressions before requesting type-dependent resolution.
    let ExprKind::MethodCall(segment, ..) = expr.kind else {
        return false;
    };
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let Some(def_id) = typeck.type_dependent_def_id(expr.hir_id) else {
        return false;
    };

    // Require both the target method name and the defining Schemars crate.
    segment.ident.name.as_str() == expected_name
        && cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
}

/// Return whether a call expression resolves to one exact Schemars associated function.
///
/// Only direct path calls are considered because closures and method calls have
/// different resolution contracts and are handled by separate helpers.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_name| {
///     let _ = schemars_support::is_schemars_function_call(cx, expr, expected_name);
/// };
/// ```
pub fn is_schemars_function_call(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected_name: &str,
) -> bool {
    // Resolve only direct call paths because closures and method calls use other contracts.
    let ExprKind::Call(callee, _) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let rustc_hir::def::Res::Def(_, def_id) = cx
        .tcx
        .typeck(callee.hir_id.owner.def_id)
        .qpath_res(path, callee.hir_id)
    else {
        return false;
    };

    // Reject local functions that reuse the same associated-function name.
    cx.tcx.crate_name(def_id.krate).as_str() == "schemars"
        && cx.tcx.item_name(def_id).as_str() == expected_name
}

/// Collect local ADTs with a semantically resolved `JsonSchema` implementation.
///
/// The result contains unique local definition identifiers in traversal order,
/// allowing callers to inspect only application types that implement the target.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx| {
///     let _ = schemars_support::local_json_schema_impls(cx);
/// };
/// ```
pub fn local_json_schema_impls(cx: &LateContext<'_>) -> Vec<LocalDefId> {
    // Accumulate unique local types while walking all local trait implementations.
    let mut implementations = Vec::new();

    // Ignore every trait except Schemars's resolved JsonSchema definition.
    for (&trait_def_id, impl_def_ids) in cx.tcx.all_local_trait_impls(()) {
        if !is_json_schema_trait(cx, trait_def_id) {
            continue;
        }

        // Retain only local ADT self types because external types cannot be lint targets.
        for &impl_def_id in impl_def_ids {
            if !matches!(
                cx.tcx.def_kind(impl_def_id),
                rustc_hir::def::DefKind::Impl { .. }
            ) {
                continue;
            }
            let self_ty = cx
                .tcx
                .type_of(impl_def_id)
                .instantiate_identity()
                .skip_norm_wip();
            let ty::Adt(adt, _) = self_ty.kind() else {
                continue;
            };
            if let Some(local_def_id) = adt.did().as_local()
                && !implementations.contains(&local_def_id)
            {
                implementations.push(local_def_id);
            }
        }
    }

    implementations
}
