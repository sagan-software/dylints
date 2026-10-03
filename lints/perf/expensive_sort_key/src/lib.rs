#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint ignores diagnostic builder results and unsupported HIR forms"
)]

//! A lint for known allocating standard-library sort-key callbacks.
//!
//! It resolves stable slice sorting and a closed set of deterministic string
//! transformations before recommending cached key evaluation.
//! The matcher accepts direct closure-argument projections and resolved standard
//! string methods; helper calls, custom receiver types, and unknown callbacks stay
//! outside the rule. UI fixtures document those boundaries, while unit tests compare
//! stable order and callback counts on representative and degenerate inputs.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{BindingMode, Expr, ExprKind, HirId, PatKind, UnOp};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::sym;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub EXPENSIVE_SORT_KEY,
    Warn,
    "sort key may allocate a string during repeated comparisons",
    ExpensiveSortKey
}

impl<'tcx> LateLintPass<'tcx> for ExpensiveSortKey {
    /// Check standard stable slice sorting for a recognized allocating key callback.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if expr.span.from_expansion() {
            return;
        }
        let ExprKind::MethodCall(_, receiver, [key], _) = expr.kind else {
            return;
        };

        // Resolve the method and closure before reporting so local lookalikes and cached sorts
        // do not receive the recommendation.
        if !is_standard_sort_by_key(cx, expr)
            || fixed_array_has_at_most_two_elements(cx, receiver)
            || !has_recognized_allocating_key(cx, key)
        {
            return;
        }

        cx.emit_span_lint(
            EXPENSIVE_SORT_KEY,
            expr.span,
            DiagDecorator(|diag| {
                let _ =
                    diag.primary_message("sort key may allocate a string during repeated comparisons");
                let _ = diag.span_label(receiver.span, "standard stable slice sorting");
                let _ = diag.help(
                    "use `sort_by_cached_key` to compute each recognized key at most once per element",
                );
            }),
        );
    }
}

/// Return whether the receiver is a statically known array of at most two elements.
fn fixed_array_has_at_most_two_elements(cx: &LateContext<'_>, receiver: &Expr<'_>) -> bool {
    matches!(cx.typeck_results().expr_ty(receiver).peel_refs().kind(),
        ty::Array(_, length) if length
            .try_to_target_usize(cx.tcx)
            .is_some_and(|length| length <= 2))
}

/// Return whether a call resolves to the standard stable slice `sort_by_key` method.
fn is_standard_sort_by_key(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let Some(def_id) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    if cx.tcx.item_name(def_id).as_str() != "sort_by_key" {
        return false;
    }

    // Only the inherent slice method has the cached stable counterpart named by the help.
    cx.tcx
        .inherent_impl_of_assoc(def_id)
        .is_some_and(|impl_id| {
            matches!(
                cx.tcx
                    .type_of(impl_id)
                    .instantiate_identity()
                    .skip_norm_wip()
                    .kind(),
                ty::Slice(_)
            )
        })
}

/// Return whether a closure consists of one recognized deterministic allocation.
fn has_recognized_allocating_key(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Restrict the analysis to closures because function items can hide callback behavior.
    let ExprKind::Closure(closure) = expr.kind else {
        return false;
    };
    let body = cx.tcx.hir_body(closure.body);

    // A standard sort callback has exactly one plain binding parameter.
    let [parameter] = body.params else {
        return false;
    };
    let PatKind::Binding(BindingMode::NONE, parameter_id, _, None) = parameter.pat.kind else {
        return false;
    };
    let key = closure_tail(body.value);
    let ExprKind::MethodCall(_, receiver, [], _) = key.kind else {
        return false;
    };

    // A single field projection from the closure argument excludes helper calls, indexing,
    // side effects, and arbitrary methods from the recognized key shape.
    is_argument_projection(cx, receiver, parameter_id)
        && cx
            .typeck_results()
            .type_dependent_def_id(key.hir_id)
            .is_some_and(|method_id| is_recognized_string_allocation(cx, key, receiver, method_id))
}

/// Return the tail of an expression or a statement-free closure block.
fn closure_tail<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::Block(block, _) if block.stmts.is_empty() => {
            block.expr.map_or(expr, closure_tail)
        }
        ExprKind::DropTemps(inner) => closure_tail(inner),
        _ => expr,
    }
}

/// Return whether an expression is a direct path or field projection from
/// the closure argument.
fn is_argument_projection(cx: &LateContext<'_>, expr: &Expr<'_>, parameter_id: HirId) -> bool {
    match expr.kind {
        ExprKind::Path(qpath) => matches!(
            cx.typeck_results().qpath_res(&qpath, expr.hir_id),
            rustc_hir::def::Res::Local(local_id) if local_id == parameter_id
        ),
        ExprKind::Field(base, _) => is_argument_projection(cx, base, parameter_id),
        ExprKind::Unary(UnOp::Deref, base)
            if matches!(cx.typeck_results().expr_ty(base).kind(), ty::Ref(..)) =>
        {
            is_argument_projection(cx, base, parameter_id)
        }
        _ => false,
    }
}

/// Classify only standard string methods that allocate a deterministic owned key.
fn is_recognized_string_allocation(
    cx: &LateContext<'_>,
    call: &Expr<'_>,
    receiver: &Expr<'_>,
    method_id: rustc_span::def_id::DefId,
) -> bool {
    let method_name = cx.tcx.item_name(method_id);

    // Exact result and receiver types keep similarly named user methods outside the rule.
    let result_is_string = is_std_string(cx, cx.typeck_results().expr_ty(call));
    if !result_is_string {
        return false;
    }

    let string_receiver = is_std_str_or_string(cx, cx.typeck_results().expr_ty(receiver));
    let is_inherent_string_case = string_receiver
        && matches!(
            method_name.as_str(),
            "to_lowercase" | "to_uppercase" | "to_ascii_lowercase" | "to_ascii_uppercase"
        )
        && cx
            .tcx
            .inherent_impl_of_assoc(method_id)
            .is_some_and(|impl_id| {
                matches!(
                    cx.tcx
                        .type_of(impl_id)
                        .instantiate_identity()
                        .skip_norm_wip()
                        .kind(),
                    ty::Str
                )
            });

    // `String::clone` is deterministic and allocates an independent owned key.
    let is_string_clone = method_name.as_str() == "clone"
        && cx
            .tcx
            .trait_of_assoc(method_id)
            .and_then(|trait_id| cx.tcx.get_diagnostic_name(trait_id))
            == Some(sym::Clone)
        && is_std_string(cx, cx.typeck_results().expr_ty(receiver));

    is_inherent_string_case || is_string_clone
}

/// Return whether a type resolves to the standard library's owned `String`.
fn is_std_string(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if matches!(cx.tcx.def_path_str(adt.did()).as_str(),
            "alloc::string::String" | "std::string::String"))
}

/// Return whether a type is `str` or standard `String` after removing references.
fn is_std_str_or_string(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty = ty.peel_refs();
    matches!(ty.kind(), ty::Str) || is_std_string(cx, ty)
}

#[cfg(test)]
mod tests {
    /// Produce the allocating key used to compare repeated and cached evaluation.
    fn lowercase_name(name: &str) -> String {
        name.to_lowercase()
    }

    /// Compare normal and cached key evaluation on an allocation-backed string key.
    #[test]
    fn cached_sort_computes_each_lowercase_key_once_and_preserves_order() {
        // Reverse labels create enough distinct comparisons to exercise repeated key calls.
        let names = (0..128)
            .rev()
            .map(|index| format!("label with enough bytes {index:03}"))
            .collect::<Vec<_>>();
        // Count calls for each stable sorting strategy on identical input order.
        let mut repeated = names.clone();
        let mut repeated_key_calls = 0;
        repeated.sort_by_key(|name| {
            repeated_key_calls += 1;
            lowercase_name(name)
        });

        let mut cached = names;
        let mut cached_key_calls = 0;
        cached.sort_by_cached_key(|name| {
            cached_key_calls += 1;
            lowercase_name(name)
        });

        assert_eq!(repeated, cached);
        assert!(repeated_key_calls > cached_key_calls);
        assert_eq!(cached_key_calls, cached.len());
    }

    /// Compare key evaluation for empty and singleton sort inputs.
    fn assert_degenerate_sorts_skip_key(source: Vec<String>) {
        let expected = source.clone();

        // Exercise ordinary stable sorting while counting whether it invokes the key.
        let mut repeated = source.clone();
        let mut repeated_key_calls = 0;
        repeated.sort_by_key(|name| {
            repeated_key_calls += 1;
            lowercase_name(name)
        });

        // Exercise cached stable sorting with the same input and call counter.
        let mut cached = source;
        let mut cached_key_calls = 0;
        cached.sort_by_cached_key(|name| {
            cached_key_calls += 1;
            lowercase_name(name)
        });

        assert_eq!((repeated, cached), (expected.clone(), expected));
        assert_eq!((repeated_key_calls, cached_key_calls), (0, 0));
    }

    /// Confirm that an empty input preserves order without evaluating keys.
    #[test]
    fn empty_sorts_skip_key_evaluation() {
        assert_degenerate_sorts_skip_key(Vec::new());
    }

    /// Confirm that a singleton input preserves order without evaluating keys.
    #[test]
    fn singleton_sorts_skip_key_evaluation() {
        assert_degenerate_sorts_skip_key(vec![String::from("solo")]);
    }

    /// Confirm that caching does not reduce key calls for a two-element input.
    #[test]
    fn two_element_sort_evaluates_each_key_once() {
        let original = [String::from("B"), String::from("a")];

        // Compare repeated and cached callbacks on the same two-element input.
        let mut repeated = original.clone();
        let mut repeated_key_calls = 0;
        repeated.sort_by_key(|name| {
            repeated_key_calls += 1;
            lowercase_name(name)
        });

        let mut cached = original;
        let mut cached_key_calls = 0;
        cached.sort_by_cached_key(|name| {
            cached_key_calls += 1;
            lowercase_name(name)
        });

        // Keep sorted output and exact callback counts as separate evidence.
        assert_eq!(repeated, cached);
        assert_eq!(repeated_key_calls, 2);
        assert_eq!(cached_key_calls, 2);
    }

    /// Preserve input order among distinct strings with equal lowercase keys.
    #[test]
    fn cached_sort_preserves_equal_key_order() {
        let original = ["A", "a", "B", "b"].map(String::from).to_vec();

        // Adjacent case variants have equal lowercase keys and expose stable tie ordering.
        let mut repeated = original.clone();
        let mut repeated_key_calls = 0;
        repeated.sort_by_key(|name| {
            repeated_key_calls += 1;
            lowercase_name(name)
        });

        // Count calls separately because caching must preserve order while reducing work.
        let mut cached = original.clone();
        let mut cached_key_calls = 0;
        cached.sort_by_cached_key(|name| {
            cached_key_calls += 1;
            lowercase_name(name)
        });

        // Both stable algorithms preserve the original equal-key order.
        assert_eq!((repeated, cached), (original.clone(), original.clone()));
        assert_eq!(cached_key_calls, original.len());
        assert!(repeated_key_calls > cached_key_calls);
    }
}

/// Run the lint's compiletest UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
