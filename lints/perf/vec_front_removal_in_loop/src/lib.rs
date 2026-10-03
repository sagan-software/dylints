#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint ignores diagnostic builder results and unsupported HIR forms"
)]

//! A lint for repeated front removal from `Vec` inside loops.
//!
//! It resolves standard `Vec::remove(0)` calls on a persistent local vector
//! and checks enclosing loop bounds before recommending a queue or consuming
//! iteration.
//! The analysis excludes known zero-sized elements, fresh vectors, and loops
//! proven to execute at most once. UI fixtures cover resolved calls and loop
//! shapes; unit tests measure preserved order, shifted elements, and signed
//! literal-bound behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../../complexity/chaining_lint_support.rs"]
mod support;

use rustc_ast::ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Expr, ExprKind, HirId, LoopSource, Node, StmtKind, UnOp,
    def::DefKind,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty, layout::LayoutOf};
use rustc_span::{Span, sym};
use std::cmp::Ordering;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub VEC_FRONT_REMOVAL_IN_LOOP,
    Warn,
    "standard `Vec::remove(0)` occurs in a loop",
    VecFrontRemovalInLoop
}

impl<'tcx> LateLintPass<'tcx> for VecFrontRemovalInLoop {
    /// Check resolved method calls for repeated removal from the front of a vector.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Keep the rule narrow: only a source-written standard call with literal index zero
        // can establish the per-call shifting cost this diagnostic describes.
        if expr.span.from_expansion() {
            return;
        }
        let Some(receiver) = vec_remove_zero_receiver(cx, expr) else {
            return;
        };

        // Resolved `Vec::remove` returns T, so query the element layout; unknown layouts stay conservative.
        if cx
            .layout_of(cx.typeck_results().expr_ty(expr))
            .is_ok_and(|layout| layout.size.bytes() == 0)
        {
            return;
        }

        // Stop at a closure boundary so a closure merely created inside a loop is not treated
        // as though the loop invokes its body.
        let Some(loop_info) = enclosing_repeating_loop(cx, expr, receiver) else {
            return;
        };

        cx.emit_span_lint(
            VEC_FRONT_REMOVAL_IN_LOOP,
            expr.span,
            DiagDecorator(|diag| {
                let _ = diag.primary_message(
                    "front removal from a `Vec` shifts the remaining elements inside a loop",
                );
                let _ = diag.span_label(loop_info.span, "this loop may remove multiple elements");
                let _ = diag.help(
                    "if this loop repeatedly removes elements, consume the vector or use `VecDeque::pop_front`",
                );
            }),
        );
    }
}

/// The nearest source loop whose body can execute more than once.
struct RepeatingLoop {
    /// Source span of the loop that contains the removal call.
    span: Span,
}

/// Return whether an expression is the standard `Vec::remove(0)` method call.
fn vec_remove_zero_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    let ExprKind::MethodCall(_, receiver, [index], _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;

    // Match both the resolved inherent method and the receiver type so local methods named
    // `remove` and custom wrappers around `Vec` stay outside the lint.
    let is_vec_remove = cx.tcx.item_name(def_id).as_str() == "remove"
        && cx
            .tcx
            .inherent_impl_of_assoc(def_id)
            .is_some_and(|impl_id| {
                is_std_vec(
                    cx,
                    cx.tcx
                        .type_of(impl_id)
                        .instantiate_identity()
                        .skip_norm_wip(),
                )
            })
        && is_std_vec(cx, cx.typeck_results().expr_ty(receiver))
        && is_literal_zero(index);
    is_vec_remove
        .then(|| direct_local_binding(cx, receiver))
        .flatten()
}

/// Return whether a type is `Vec<T>` after removing reference layers.
fn is_std_vec(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Vec, adt.did()))
}

/// Return whether an index expression is the literal integer zero.
fn is_literal_zero(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal)
        if matches!(literal.node, LitKind::Int(value, _) if value.get() == 0))
}

/// Return the resolved local binding for a direct vector receiver.
fn direct_local_binding(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    let expr = support::peel_drop_temps(expr);
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };
    match cx.typeck_results().qpath_res(&qpath, expr.hir_id) {
        rustc_hir::def::Res::Local(binding) => Some(binding),
        _ => None,
    }
}

/// Find a repeated enclosing loop without crossing an uninvoked closure body.
fn enclosing_repeating_loop<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    receiver: HirId,
) -> Option<RepeatingLoop> {
    // Search outward from the call so the nearest eligible loop controls the warning.
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        // Non-expression parents cannot describe an enclosing loop body.
        let Node::Expr(parent) = node else {
            continue;
        };

        // Stop at closure creation because the closure may never execute.
        if matches!(parent.kind, ExprKind::Closure(..)) {
            return None;
        }

        let ExprKind::Loop(body, _label, source, span) = parent.kind else {
            continue;
        };
        if loop_can_repeat(cx, parent, body, receiver, source) {
            return Some(RepeatingLoop {
                span: span.source_callsite(),
            });
        }
    }
    None
}

/// Return whether a loop can execute its body more than once.
fn loop_can_repeat<'tcx>(
    cx: &LateContext<'tcx>,
    loop_expr: &'tcx Expr<'tcx>,
    body: &'tcx rustc_hir::Block<'tcx>,
    receiver: HirId,
    source: LoopSource,
) -> bool {
    let loop_id = loop_expr.hir_id;

    // A vector declared inside the loop is a fresh collection per iteration.
    let receiver_binding_is_inside_loop = cx
        .tcx
        .hir_parent_iter(receiver)
        .any(|(_, node)| matches!(node, Node::Expr(expr) if expr.hir_id == loop_id));
    if receiver_binding_is_inside_loop || has_loop_reassignment(cx, loop_expr, receiver) {
        return false;
    }

    // A direct final break proves this loop exits after one body execution.
    if block_ends_with_target_break(body, loop_id) {
        return false;
    }

    // Literal singleton arrays and ranges prove that a standard `for` body runs at most once.
    if source == LoopSource::ForLoop {
        let for_source = cx
            .tcx
            .hir_parent_iter(loop_expr.hir_id)
            .find_map(|(_, node)| {
                let Node::Expr(parent) = node else {
                    return None;
                };
                support::for_loop(cx, parent).map(|for_loop| for_loop.source)
            });
        return !for_source.is_some_and(|source| for_source_has_at_most_one_item(cx, source));
    }

    // A while or unconditional loop may repeat unless the direct final break
    // above proves otherwise.
    true
}

/// Return whether a recognized standard `for` source contains at most one item.
fn for_source_has_at_most_one_item(cx: &LateContext<'_>, source: &Expr<'_>) -> bool {
    match source.kind {
        ExprKind::Array(items) => items.len() <= 1,
        ExprKind::Struct(_, _, _) | ExprKind::Call(_, _) if is_std_range(cx, source) => {
            literal_range_has_at_most_one_item(cx, source)
        }
        _ => false,
    }
}

/// Return whether a loop body assigns a new value directly to the vector binding.
fn has_loop_reassignment<'tcx>(
    cx: &LateContext<'tcx>,
    loop_expr: &'tcx Expr<'tcx>,
    binding: HirId,
) -> bool {
    let mut visitor = ReassignmentFinder {
        cx,
        binding,
        has_reassignment: false,
    };
    visitor.visit_expr(loop_expr);
    visitor.has_reassignment
}

/// Visitor that finds direct assignments to the vector binding inside one loop.
struct ReassignmentFinder<'cx, 'tcx> {
    /// Type and name resolution for local places.
    cx: &'cx LateContext<'tcx>,
    /// Binding whose collection must persist between iterations.
    binding: HirId,
    /// Set after a direct assignment to the binding is found.
    has_reassignment: bool,
}

impl<'tcx> Visitor<'tcx> for ReassignmentFinder<'_, 'tcx> {
    type NestedFilter = rustc_middle::hir::nested_filter::OnlyBodies;

    /// Provide the type context required to inspect nested closure bodies conservatively.
    fn maybe_tcx(&mut self) -> ty::TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Look for writes while continuing through all body expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Record direct writes before walking child and nested-body expressions.
        let assigned = match expr.kind {
            ExprKind::Assign(lhs, _, _) | ExprKind::AssignOp(_, lhs, _) => {
                assignment_root_binding(self.cx, lhs)
            }
            _ => None,
        };
        if assigned == Some(self.binding) {
            self.has_reassignment = true;
        }
        // Continue walking so assignments in nested closure bodies are included.
        walk_expr(self, expr);
    }
}

/// Return the direct binding assigned by a local or single dereference place.
fn assignment_root_binding(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    match expr.kind {
        ExprKind::Unary(UnOp::Deref, base) => direct_local_binding(cx, base),
        _ => direct_local_binding(cx, expr),
    }
}

/// Return whether an expression resolves to the standard inclusive or exclusive range.
fn is_std_range(cx: &LateContext<'_>, source: &Expr<'_>) -> bool {
    is_std_range_type(cx, cx.typeck_results().expr_ty(source))
}

/// Return whether a type is one of the standard range types.
fn is_std_range_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if matches!(cx.tcx.def_path_str(adt.did()).as_str(),
            "core::ops::range::Range" | "core::ops::range::RangeInclusive"
                | "std::ops::Range" | "std::ops::RangeInclusive"))
}

/// Return whether a type is the standard inclusive range type.
fn is_std_inclusive_range_type(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if matches!(cx.tcx.def_path_str(adt.did()).as_str(),
            "core::ops::range::RangeInclusive" | "std::ops::RangeInclusive"))
}

/// Extract direct literal bounds from a standard exclusive or inclusive range expression.
fn literal_range_bounds(
    cx: &LateContext<'_>,
    source: &Expr<'_>,
) -> Option<(LiteralInteger, LiteralInteger, bool)> {
    // Read direct bounds only; evaluating arbitrary range expressions could
    // change behavior.
    match source.kind {
        ExprKind::Struct(_, fields, _) => {
            let start = fields
                .iter()
                .find(|field| field.ident.name.as_str() == "start")
                .and_then(|field| literal_integer(field.expr));
            let end = fields
                .iter()
                .find(|field| field.ident.name.as_str() == "end")
                .and_then(|field| literal_integer(field.expr));
            Some((
                start?,
                end?,
                is_std_inclusive_range_type(cx, cx.typeck_results().expr_ty(source)),
            ))
        }
        ExprKind::Call(callee, [start, end])
            if is_std_inclusive_range_type(cx, cx.typeck_results().expr_ty(source))
                && is_std_inclusive_range_constructor(cx, callee) =>
        {
            Some((literal_integer(start)?, literal_integer(end)?, true))
        }
        _ => None,
    }
}

/// Match the resolved standard `RangeInclusive::new` constructor.
fn is_std_inclusive_range_constructor(cx: &LateContext<'_>, callee: &Expr<'_>) -> bool {
    // Resolve the associated function before checking its inherent implementation type.
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let rustc_hir::def::Res::Def(DefKind::AssocFn, def_id) =
        cx.typeck_results().qpath_res(&qpath, callee.hir_id)
    else {
        return false;
    };
    cx.tcx.item_name(def_id).as_str() == "new"
        && cx
            .tcx
            .inherent_impl_of_assoc(def_id)
            .is_some_and(|impl_id| {
                is_std_inclusive_range_type(
                    cx,
                    cx.tcx
                        .type_of(impl_id)
                        .instantiate_identity()
                        .skip_norm_wip(),
                )
            })
}

/// Check whether a standard literal range has a cardinality no greater than one.
fn literal_range_has_at_most_one_item(cx: &LateContext<'_>, source: &Expr<'_>) -> bool {
    let Some((start, end, is_inclusive)) = literal_range_bounds(cx, source) else {
        return false;
    };
    match end.cmp(&start) {
        Ordering::Less | Ordering::Equal => true,
        Ordering::Greater if is_inclusive => false,
        Ordering::Greater => start.is_at_most_one_apart_from(end),
    }
}

/// A signed literal represented without overflowing at the minimum integer value.
#[derive(Clone, Copy)]
struct LiteralInteger {
    /// Whether this value is below zero.
    is_negative: bool,
    /// Absolute value, including for the minimum signed integer.
    magnitude: u128,
}

impl LiteralInteger {
    /// Compare signed values while retaining the full unsigned literal range.
    fn cmp(&self, other: &Self) -> Ordering {
        match (self.is_negative, other.is_negative) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => other.magnitude.cmp(&self.magnitude),
            (false, false) => self.magnitude.cmp(&other.magnitude),
        }
    }

    /// Return whether ordered bounds differ by at most one.
    fn is_at_most_one_apart_from(self, upper: Self) -> bool {
        match (self.is_negative, upper.is_negative) {
            (false, false) => upper.magnitude - self.magnitude <= 1,
            (true, true) => self.magnitude - upper.magnitude <= 1,
            (true, false) => self
                .magnitude
                .checked_add(upper.magnitude)
                .is_some_and(|distance| distance <= 1),
            (false, true) => false,
        }
    }
}

/// Return a direct integer literal or a negated integer literal.
fn literal_integer(expr: &Expr<'_>) -> Option<LiteralInteger> {
    // Keep sign and unsigned magnitude separate so minimum signed values do not overflow.
    match expr.kind {
        ExprKind::Lit(literal) => match literal.node {
            LitKind::Int(value, _) => Some(LiteralInteger {
                is_negative: false,
                magnitude: value.get(),
            }),
            _ => None,
        },
        ExprKind::Unary(UnOp::Neg, operand) => {
            // Accept only a direct integer operand and normalize negative zero.
            let ExprKind::Lit(literal) = operand.kind else {
                return None;
            };
            let LitKind::Int(value, _) = literal.node else {
                return None;
            };
            let magnitude = value.get();
            Some(LiteralInteger {
                is_negative: magnitude != 0,
                magnitude,
            })
        }
        _ => None,
    }
}

/// Return whether the final direct expression or statement breaks this loop.
fn block_ends_with_target_break(block: &rustc_hir::Block<'_>, loop_id: HirId) -> bool {
    let final_expr = block.expr.or_else(|| {
        block.stmts.last().and_then(|stmt| match stmt.kind {
            StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
            _ => None,
        })
    });
    final_expr.is_some_and(|expr| {
        matches!(expr.kind, ExprKind::Break(destination, _)
            if destination.target_id == Ok(loop_id))
    })
}

#[cfg(test)]
mod tests {
    use super::{LiteralInteger, Ordering};

    /// An element used to observe whether front removal shifts its address.
    struct Item {
        /// Stable identity used to match an element before and after removal.
        id: usize,
    }

    /// Measure tail element shifts while draining a vector in order.
    fn drain_and_count_shifts(mut items: Vec<Item>) -> (Vec<usize>, usize) {
        let mut order = Vec::new();
        let mut shifted = 0;
        while !items.is_empty() {
            // Snapshot identities and addresses before removal to distinguish movement from drops.
            let before = items
                .iter()
                .map(|item| (item.id, std::ptr::from_ref(item) as usize))
                .collect::<Vec<_>>();
            let removed = remove_front_for_test(&mut items);
            order.push(removed.id);
            // Count surviving elements whose addresses changed after front removal.
            shifted += items
                .iter()
                .filter(|item| {
                    let old = before
                        .iter()
                        .find(|(id, _)| *id == item.id)
                        .map(|(_, address)| *address);
                    old.is_some_and(|address| address != std::ptr::from_ref(*item) as usize)
                })
                .count();
        }
        (order, shifted)
    }

    /// Remove one element while keeping the measured loop at the helper boundary.
    fn remove_front_for_test(items: &mut Vec<Item>) -> Item {
        items.remove(0)
    }

    /// Show that draining preserves order and moves n(n - 1) / 2 tail elements.
    #[test]
    fn front_drain_preserves_order_and_counts_shifts() {
        let items = (0..8).map(|id| Item { id }).collect();

        let (order, shifted) = drain_and_count_shifts(items);

        assert_eq!(order, (0..8).collect::<Vec<_>>());
        assert_eq!(shifted, 28);
    }

    /// Confirm that zero or one item requires no tail shifts.
    #[test]
    fn empty_and_singleton_vectors_do_not_shift_tail_items() {
        assert_eq!(drain_and_count_shifts(Vec::new()), (Vec::new(), 0));
        assert_eq!(drain_and_count_shifts(vec![Item { id: 7 }]), (vec![7], 0));
    }

    /// Check signed-bound ordering across signs and magnitudes.
    #[test]
    fn signed_literal_bounds_compare_signs_and_magnitudes() {
        // Cover both sign transitions and magnitude ordering within each sign.
        let negative_two = LiteralInteger {
            is_negative: true,
            magnitude: 2,
        };
        let negative_one = LiteralInteger {
            is_negative: true,
            magnitude: 1,
        };
        let zero = LiteralInteger {
            is_negative: false,
            magnitude: 0,
        };
        let one = LiteralInteger {
            is_negative: false,
            magnitude: 1,
        };
        let three = LiteralInteger {
            is_negative: false,
            magnitude: 3,
        };

        // Compare both cross-sign directions and same-sign magnitude order.
        assert_eq!(
            (
                negative_two.cmp(&zero),
                zero.cmp(&negative_two),
                negative_two.cmp(&negative_one),
                one.cmp(&three),
            ),
            (
                Ordering::Less,
                Ordering::Greater,
                Ordering::Less,
                Ordering::Less,
            )
        );
    }

    /// Check adjacent and separated ranges across signed bounds.
    #[test]
    fn signed_literal_bounds_measure_small_distances() {
        // Use representative negative, zero, positive, and distant bounds.
        let negative_two = LiteralInteger {
            is_negative: true,
            magnitude: 2,
        };
        let negative_one = LiteralInteger {
            is_negative: true,
            magnitude: 1,
        };
        let zero = LiteralInteger {
            is_negative: false,
            magnitude: 0,
        };
        let one = LiteralInteger {
            is_negative: false,
            magnitude: 1,
        };
        let three = LiteralInteger {
            is_negative: false,
            magnitude: 3,
        };

        // Check adjacent values on both sides of zero and reject larger gaps.
        assert_eq!(
            (
                negative_two.is_at_most_one_apart_from(negative_one),
                negative_one.is_at_most_one_apart_from(zero),
                zero.is_at_most_one_apart_from(one),
                negative_two.is_at_most_one_apart_from(zero),
                one.is_at_most_one_apart_from(three),
            ),
            (true, true, true, false, false)
        );
    }

    /// Check cross-sign addition overflow at the full unsigned magnitude.
    #[test]
    fn signed_literal_cross_sign_distance_checks_overflow() {
        // Largest opposite-sign magnitudes exercise checked addition without signed casts.
        let negative_max = LiteralInteger {
            is_negative: true,
            magnitude: u128::MAX,
        };
        let positive_max = LiteralInteger {
            is_negative: false,
            magnitude: u128::MAX,
        };
        let last = LiteralInteger {
            is_negative: false,
            magnitude: u128::MAX,
        };
        let previous = LiteralInteger {
            magnitude: u128::MAX - 1,
            ..last
        };

        // Overflow is rejected while adjacent upper-bound values remain valid.
        assert_eq!(
            (
                negative_max.is_at_most_one_apart_from(positive_max),
                previous.is_at_most_one_apart_from(last),
            ),
            (false, true)
        );
    }
}

/// Run the lint's compiletest UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
