#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores future rustc expression variants"
)]

//! A lint for a separate `contains_key` call before `Iterator::any`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    BinOpKind, BindingMode, ByRef, Expr, ExprKind, HirId, Mutability, Pat, PatKind, UnOp, def::Res,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::{Span, Symbol, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CONTAINS_KEY_BEFORE_ANY,
    Warn,
    "a separate `contains_key` call can be folded into the array searched by `any`",
    ContainsKeyBeforeAny
}

impl<'tcx> LateLintPass<'tcx> for ContainsKeyBeforeAny {
    /// Checks one disjunction for a `contains_key` call that can join the array.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(candidate) = validate_contains_key_candidate(cx, expr) else {
            return;
        };
        let suggestion = fold_suggestion(cx, expr, &candidate);
        // Report the full disjunction that can become one literal array search.
        cx.emit_span_lint(
            CONTAINS_KEY_BEFORE_ANY,
            expr.span,
            DiagDecorator(|diagnostic| {
                let help =
                    "include the separate string literal in the array and remove this `||` gate";
                let diagnostic = diagnostic.primary_message(
                    "a separate `contains_key` call can be folded into the array searched by `any`",
                );
                if let Some(parts) = suggestion {
                    let _diagnostic = diagnostic.multipart_suggestion(
                        help,
                        parts,
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _diagnostic = diagnostic.help(help);
                }
            }),
        );
    }
}

/// Source parts of a disjunction that can be folded into one key search.
struct Candidate<'tcx> {
    /// The separate key literal passed to the first `contains_key` call.
    key: &'tcx Expr<'tcx>,
    /// The `[..].iter().any(..)` call on the right of `||`.
    any_call: &'tcx Expr<'tcx>,
    /// The first array element; the separate key is inserted before it.
    first_key: &'tcx Expr<'tcx>,
}

/// Build the rewrite that moves the separate key to the front of the array.
///
/// The folded search calls the same resolved method with the same keys in the
/// same order and stops at the same first match, so it keeps both behavior and
/// types. Macro-generated source stays on the help-only path because its text
/// cannot be edited in place.
fn fold_suggestion(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    candidate: &Candidate<'_>,
) -> Option<Vec<(Span, String)>> {
    // Keep generated source on the help-only path.
    let spans = [
        expr.span,
        candidate.key.span,
        candidate.any_call.span,
        candidate.first_key.span,
    ];
    if spans.iter().any(|span| span.from_expansion()) {
        return None;
    }
    // Reuse the literal's source text so raw strings and escapes stay unchanged.
    let key = cx
        .sess()
        .source_map()
        .span_to_snippet(candidate.key.span)
        .ok()?;
    Some(vec![
        // Remove `receiver.contains_key("key") ||` and the whitespace before the array.
        (expr.span.until(candidate.any_call.span), String::new()),
        (candidate.first_key.span.shrink_to_lo(), format!("{key}, ")),
    ])
}

/// Return the parts of a disjunction that can be folded into one key search.
fn validate_contains_key_candidate<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<Candidate<'tcx>> {
    // Prove the binary shape, literal keys, and closure identity in source order.
    match expr.kind {
        ExprKind::Binary(operation, separate_call, any_call) if operation.node == BinOpKind::Or => {
            Some((separate_call, any_call))
        }
        _ => None,
    }
    .and_then(|(separate_call, any_call)| {
        contains_key_call(cx, separate_call).map(|call| (call, any_call))
    })
    .filter(|((_, _, key), _)| is_string_literal(key))
    .and_then(|((method, receiver, key), any_call)| {
        validate_literal_array_any(cx, any_call)
            .map(|(keys, closure)| (method, receiver, key, any_call, keys, closure))
    })
    .filter(|(_, _, _, _, keys, _)| keys.iter().all(|key| is_string_literal(key)))
    .and_then(|(method, receiver, key, any_call, keys, closure)| {
        let first_key = keys.first()?;
        closure_contains_key(cx, closure).map(|call| {
            (
                method,
                receiver,
                Candidate {
                    key,
                    any_call,
                    first_key,
                },
                call,
            )
        })
    })
    .filter(
        |(method, receiver, _, (closure_method, closure_receiver, closure_key, parameter))| {
            *method == *closure_method
                && is_same_local(cx, receiver, closure_receiver)
                && is_dereferenced_local(cx, closure_key, *parameter)
        },
    )
    .map(|(_, _, candidate, _)| candidate)
}

/// Returns the resolved method, receiver, and key for one `contains_key` call.
fn contains_key_call<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(DefId, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    // Check the written name and resolved item so custom spelling cannot hide a different call.
    let ExprKind::MethodCall(segment, receiver, [key], _) = expr.kind else {
        return None;
    };
    if !is_contains_key_name(segment.ident.name.as_str()) {
        return None;
    }
    // Verify the resolved method keeps the same `contains_key` identity.
    let method = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_contains_key_name(cx.tcx.item_name(method).as_str()).then_some((method, receiver, key))
}

/// Returns whether a resolved or written method name is `contains_key`.
fn is_contains_key_name(name: &str) -> bool {
    name == "contains_key"
}

/// Returns the literal array and closure from a standard `iter().any(...)` call.
fn validate_literal_array_any<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(&'tcx [Expr<'tcx>], &'tcx Expr<'tcx>)> {
    // Restrict the fold to the standard iterator shape and an inline array.
    match expr.kind {
        ExprKind::MethodCall(_, iterator, [closure], _) => Some((iterator, closure)),
        _ => None,
    }
    .filter(|_| is_standard_iterator_method(cx, expr, "any"))
    // Resolve the receiver as an inline array followed by the core `iter` method.
    .and_then(|(iterator, closure)| match iterator.kind {
        ExprKind::MethodCall(_, keys, [], _) if is_slice_iter(cx, iterator) => {
            Some((keys, closure))
        }
        _ => None,
    })
    // Return only the inline array elements and the `any` closure.
    .and_then(|(keys, closure)| match keys.kind {
        ExprKind::Array(keys) => Some((keys, closure)),
        _ => None,
    })
}

/// Returns the repeated method call and closure parameter from the `any` closure.
fn closure_contains_key<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<(DefId, &'tcx Expr<'tcx>, &'tcx Expr<'tcx>, HirId)> {
    // Recover the single closure parameter before matching its complete body.
    let ExprKind::Closure(closure) = expr.kind else {
        return None;
    };
    // Require one direct binding and one resolved contains-key call in the body.
    let body = cx.tcx.hir_body(closure.body);
    let [parameter] = body.params else {
        return None;
    };
    let parameter = binding_id(parameter.pat)?;
    let (method, receiver, key) = contains_key_call(cx, body.value)?;
    Some((method, receiver, key, parameter))
}

/// Returns the local binding represented by a simple closure parameter.
const fn binding_id(pattern: &Pat<'_>) -> Option<HirId> {
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Not), hir_id, _, None) = pattern.kind
    else {
        return None;
    };
    Some(hir_id)
}

/// Returns whether an expression is a string literal.
const fn is_string_literal(expr: &Expr<'_>) -> bool {
    matches!(
        expr.kind,
        ExprKind::Lit(literal) if matches!(literal.node, rustc_ast::LitKind::Str(..))
    )
}

/// Returns whether a method resolves to the standard `Iterator` trait method.
fn is_standard_iterator_method(cx: &LateContext<'_>, expr: &Expr<'_>, name: &str) -> bool {
    // Trace the associated item to the diagnostic `Iterator` trait.
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    // Recover the owning trait through the associated item definition.
    let Some(associated_item) = cx.tcx.opt_associated_item(method) else {
        return false;
    };
    let Ok(trait_item) = associated_item.trait_item_or_self() else {
        return false;
    };
    let Some(trait_id) = cx.tcx.trait_of_assoc(trait_item) else {
        return false;
    };
    cx.tcx.item_name(method).as_str() == name && cx.tcx.is_diagnostic_item(sym::Iterator, trait_id)
}

/// Returns whether a method call resolves to the standard slice `iter` method.
fn is_slice_iter(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|method| {
            cx.tcx
                .is_diagnostic_item(Symbol::intern("slice_iter"), method)
        })
}

/// Returns whether two expressions resolve to the same local binding.
fn is_same_local(cx: &LateContext<'_>, first: &Expr<'_>, second: &Expr<'_>) -> bool {
    local_id(cx, first).is_some_and(|first_id| Some(first_id) == local_id(cx, second))
}

/// Returns whether an expression dereferences the target local binding binding.
fn is_dereferenced_local(cx: &LateContext<'_>, expr: &Expr<'_>, expected: HirId) -> bool {
    matches!(expr.kind, ExprKind::Unary(UnOp::Deref, inner) if local_id(cx, inner) == Some(expected))
}

/// Returns the local binding resolved by a path expression.
fn local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    // Compare resolved locals so shadowed bindings with the same spelling stay distinct.
    let ExprKind::Path(path) = expr.kind else {
        return None;
    };
    let Res::Local(hir_id) = cx.qpath_res(&path, expr.hir_id) else {
        return None;
    };
    Some(hir_id)
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

#[cfg(test)]
mod tests {
    use super::is_contains_key_name;

    /// Confirms that similarly named methods remain outside the lint contract.
    #[test]
    fn target_method_name_is_exact() {
        let results = ["contains_key", "contains", "contains_key_value"].map(is_contains_key_name);
        assert_eq!(results, [true, false, false]);
    }
}
