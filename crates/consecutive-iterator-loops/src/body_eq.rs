//! Structural equality for two loop bodies that bind corresponding patterns.
//!
//! The comparison walks both HIR trees in lockstep. A local in the first body
//! equals a local in the second body only when both resolve to the same binding
//! or to bindings introduced at the same position of the compared patterns.
//! Every compared node must also have the same type and generic arguments, so
//! two calls that resolve to different functions or instances never match.
//! Unsupported syntax compares unequal, which keeps the lint quiet.

use rustc_hir::{
    Arm, Block, Destination, Expr, ExprKind, HirId, LetStmt, Pat, PatKind, Stmt, StmtKind, def::Res,
};
use rustc_lint::LateContext;
use rustc_middle::ty::{self, GenericArg, GenericArgKind};

/// Compares two HIR trees while tracking corresponding local bindings.
pub(crate) struct BodyEq<'cx, 'tcx> {
    /// Compiler context used for name resolution and typeck results.
    cx: &'cx LateContext<'tcx>,
    /// Pairs of bindings that occupy the same position in both trees.
    locals: Vec<(HirId, HirId)>,
}

impl<'cx, 'tcx> BodyEq<'cx, 'tcx> {
    /// Creates a comparator with no corresponding bindings yet.
    pub(crate) const fn new(cx: &'cx LateContext<'tcx>) -> Self {
        Self {
            cx,
            locals: Vec::new(),
        }
    }

    /// Returns whether two patterns bind the same shape, and records their bindings.
    pub(crate) fn same_pat(&mut self, left: &Pat<'_>, right: &Pat<'_>) -> bool {
        same_pat(self, left, right)
    }

    /// Returns whether two blocks are equal modulo the recorded bindings.
    pub(crate) fn same_block(&mut self, left: &Block<'_>, right: &Block<'_>) -> bool {
        same_block(self, left, right)
    }
}

/// Requires equal pattern types before comparing their structure.
fn same_pat(eq: &mut BodyEq<'_, '_>, left: &Pat<'_>, right: &Pat<'_>) -> bool {
    same_node_type(eq, left.hir_id, right.hir_id) && same_pat_shape(eq, left, right)
}

/// Returns whether two patterns bind the same shape, and records their bindings.
fn same_pat_shape(eq: &mut BodyEq<'_, '_>, left: &Pat<'_>, right: &Pat<'_>) -> bool {
    // Compare each pattern form and record corresponding local bindings.
    match (left.kind, right.kind) {
        (PatKind::Wild, PatKind::Wild) => true,
        (
            PatKind::Binding(left_mode, left_id, _, left_sub),
            PatKind::Binding(right_mode, right_id, _, right_sub),
        ) => {
            // Pair the bindings so later uses in each body correspond.
            eq.locals.push((left_id, right_id));
            left_mode == right_mode && same_option(eq, left_sub, right_sub, same_pat)
        }
        (PatKind::Tuple(left_items, left_dots), PatKind::Tuple(right_items, right_dots)) => {
            same_pat_fields(eq, left_dots == right_dots, left_items, right_items)
        }
        (
            PatKind::TupleStruct(left_ctor, left_fields, left_dots),
            PatKind::TupleStruct(right_ctor, right_fields, right_dots),
        ) => {
            // Resolve constructors before pairing their fields by position.
            let left_res = eq.cx.qpath_res(&left_ctor, left.hir_id);
            let right_res = eq.cx.qpath_res(&right_ctor, right.hir_id);
            same_pat_fields(
                eq,
                (left_res, left_dots) == (right_res, right_dots),
                left_fields,
                right_fields,
            )
        }
        (
            PatKind::Ref(left_pat, left_pin, left_mut),
            PatKind::Ref(right_pat, right_pin, right_mut),
        ) => (left_pin, left_mut) == (right_pin, right_mut) && same_pat(eq, left_pat, right_pat),
        _ => false,
    }
}

/// Returns whether two blocks are equal statement by statement.
fn same_block(eq: &mut BodyEq<'_, '_>, left: &Block<'_>, right: &Block<'_>) -> bool {
    left.rules == right.rules
        && same_list(eq, left.stmts, right.stmts, same_stmt)
        && same_option(eq, left.expr, right.expr, same_expr)
}

/// Returns whether two statements are equal.
fn same_stmt(eq: &mut BodyEq<'_, '_>, left: &Stmt<'_>, right: &Stmt<'_>) -> bool {
    match (left.kind, right.kind) {
        (StmtKind::Let(left_let), StmtKind::Let(right_let)) => same_let(eq, left_let, right_let),
        (StmtKind::Expr(left_expr), StmtKind::Expr(right_expr))
        | (StmtKind::Semi(left_expr), StmtKind::Semi(right_expr)) => {
            same_expr(eq, left_expr, right_expr)
        }
        _ => false,
    }
}

/// Returns whether two `let` statements are equal.
fn same_let(eq: &mut BodyEq<'_, '_>, left: &LetStmt<'_>, right: &LetStmt<'_>) -> bool {
    // Require the same `super`, type-annotation, and `else` shape.
    let same_shape =
        (left.super_.is_some(), left.ty.is_some()) == (right.super_.is_some(), right.ty.is_some());
    let has_no_else = left.els.is_none() && right.els.is_none();

    // Compare the initializer before the pattern, matching evaluation and scope order.
    same_shape
        && has_no_else
        && same_option(eq, left.init, right.init, same_expr)
        && same_pat(eq, left.pat, right.pat)
}

/// Returns whether two expressions are structurally equal.
fn same_expr(eq: &mut BodyEq<'_, '_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    // Equal trees must agree on every node type and instantiation.
    same_node_args(eq, left.hir_id, right.hir_id)
        && same_node_type(eq, left.hir_id, right.hir_id)
        && same_value_expr(eq, left, right)
}

/// Compares paths, literals, calls, tuples, and arrays.
fn same_value_expr(eq: &mut BodyEq<'_, '_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    // Compare leaf values and calls before descending into operators.
    match (left.kind, right.kind) {
        (ExprKind::Path(left_path), ExprKind::Path(right_path)) => same_res(
            eq,
            eq.cx.qpath_res(&left_path, left.hir_id),
            eq.cx.qpath_res(&right_path, right.hir_id),
        ),
        (ExprKind::Lit(left_lit), ExprKind::Lit(right_lit)) => left_lit.node == right_lit.node,
        (ExprKind::Call(left_fn, left_args), ExprKind::Call(right_fn, right_args)) => {
            same_operands(eq, true, &[(left_fn, right_fn)])
                && same_list(eq, left_args, right_args, same_expr)
        }
        (
            ExprKind::MethodCall(_, left_receiver, left_args, _),
            ExprKind::MethodCall(_, right_receiver, right_args, _),
        ) => {
            // Compare the resolved method, then its receiver and arguments.
            let typeck = eq.cx.typeck_results();
            let same_method = typeck.type_dependent_def_id(left.hir_id)
                == typeck.type_dependent_def_id(right.hir_id);
            same_operands(eq, same_method, &[(left_receiver, right_receiver)])
                && same_list(eq, left_args, right_args, same_expr)
        }
        (ExprKind::Tup(left_items), ExprKind::Tup(right_items))
        | (ExprKind::Array(left_items), ExprKind::Array(right_items)) => {
            same_list(eq, left_items, right_items, same_expr)
        }
        _ => same_operator_expr(eq, left, right),
    }
}

/// Compares operators, borrows, projections, and assignments.
fn same_operator_expr(eq: &mut BodyEq<'_, '_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    match (left.kind, right.kind) {
        (
            ExprKind::Binary(left_op, left_lhs, left_rhs),
            ExprKind::Binary(right_op, right_lhs, right_rhs),
        ) => same_operands(
            eq,
            left_op.node == right_op.node,
            &[(left_lhs, right_lhs), (left_rhs, right_rhs)],
        ),
        (ExprKind::Unary(left_op, left_inner), ExprKind::Unary(right_op, right_inner)) => {
            same_operands(eq, left_op == right_op, &[(left_inner, right_inner)])
        }
        (
            ExprKind::AddrOf(left_kind, left_mut, left_inner),
            ExprKind::AddrOf(right_kind, right_mut, right_inner),
        ) => same_operands(
            eq,
            (left_kind, left_mut) == (right_kind, right_mut),
            &[(left_inner, right_inner)],
        ),
        (ExprKind::Field(left_base, left_name), ExprKind::Field(right_base, right_name)) => {
            same_operands(
                eq,
                left_name.name == right_name.name,
                &[(left_base, right_base)],
            )
        }
        (
            ExprKind::Index(left_base, left_index, _),
            ExprKind::Index(right_base, right_index, _),
        )
        | (
            ExprKind::Assign(left_base, left_index, _),
            ExprKind::Assign(right_base, right_index, _),
        ) => same_operands(
            eq,
            true,
            &[(left_base, right_base), (left_index, right_index)],
        ),
        (
            ExprKind::AssignOp(left_op, left_place, left_value),
            ExprKind::AssignOp(right_op, right_place, right_value),
        ) => same_operands(
            eq,
            left_op.node == right_op.node,
            &[(left_place, right_place), (left_value, right_value)],
        ),
        _ => same_control_expr(eq, left, right),
    }
}

/// Compares blocks, conditionals, matches, closures, and supported jumps.
fn same_control_expr(eq: &mut BodyEq<'_, '_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    // Compare block order and every branch of conditionals and matches.
    match (left.kind, right.kind) {
        (ExprKind::Block(left_block, None), ExprKind::Block(right_block, None)) => {
            same_block(eq, left_block, right_block)
        }
        (
            ExprKind::If(left_cond, left_then, left_else),
            ExprKind::If(right_cond, right_then, right_else),
        ) => {
            same_operands(
                eq,
                true,
                &[(left_cond, right_cond), (left_then, right_then)],
            ) && same_option(eq, left_else, right_else, same_expr)
        }
        (
            ExprKind::Match(left_scrutinee, left_arms, left_source),
            ExprKind::Match(right_scrutinee, right_arms, right_source),
        ) => {
            same_operands(
                eq,
                left_source == right_source,
                &[(left_scrutinee, right_scrutinee)],
            ) && same_list(eq, left_arms, right_arms, same_arm)
        }
        _ => same_closure_or_jump(eq, left, right),
    }
}

/// Compares closure captures and bodies or supported jumps.
fn same_closure_or_jump(eq: &mut BodyEq<'_, '_>, left: &Expr<'_>, right: &Expr<'_>) -> bool {
    // Closures compare their captures, parameters, and complete bodies.
    match (left.kind, right.kind) {
        (ExprKind::Closure(left_closure), ExprKind::Closure(right_closure)) => {
            let left_body = eq.cx.tcx.hir_body(left_closure.body);
            let right_body = eq.cx.tcx.hir_body(right_closure.body);
            left_closure.capture_clause == right_closure.capture_clause
                && same_list(
                    eq,
                    left_body.params,
                    right_body.params,
                    |eq, left, right| same_pat(eq, left.pat, right.pat),
                )
                && same_operands(eq, true, &[(left_body.value, right_body.value)])
        }
        // Return values must agree; unlabeled continues target the compared loops.
        (ExprKind::Ret(left_value), ExprKind::Ret(right_value)) => {
            same_option(eq, left_value, right_value, same_expr)
        }
        (
            ExprKind::Continue(Destination { label: None, .. }),
            ExprKind::Continue(Destination { label: None, .. }),
        ) => true,
        _ => false,
    }
}

/// Compares positional pattern fields when their constructor and rest position agree.
fn same_pat_fields(
    eq: &mut BodyEq<'_, '_>,
    has_same_shape: bool,
    left: &[Pat<'_>],
    right: &[Pat<'_>],
) -> bool {
    has_same_shape && same_list(eq, left, right, same_pat)
}

/// Returns whether two match arms are equal, comparing the pattern before its uses.
fn same_arm(eq: &mut BodyEq<'_, '_>, left: &Arm<'_>, right: &Arm<'_>) -> bool {
    same_pat(eq, left.pat, right.pat)
        && same_option(eq, left.guard, right.guard, same_expr)
        && same_operands(eq, true, &[(left.body, right.body)])
}

/// Returns whether a node-local check holds and every operand pair is equal.
fn same_operands(
    eq: &mut BodyEq<'_, '_>,
    same_node: bool,
    operands: &[(&Expr<'_>, &Expr<'_>)],
) -> bool {
    same_node
        && operands
            .iter()
            .all(|(left, right)| same_expr(eq, left, right))
}

/// Returns whether two resolutions name the same or corresponding item.
fn same_res(eq: &BodyEq<'_, '_>, left: Res, right: Res) -> bool {
    match (left, right) {
        (Res::Local(left_id), Res::Local(right_id)) => {
            left_id == right_id || eq.locals.contains(&(left_id, right_id))
        }
        _ => left == right && left != Res::Err,
    }
}

/// Returns whether two lists have the same length and pairwise equal items.
fn same_list<T>(
    eq: &mut BodyEq<'_, '_>,
    left: &[T],
    right: &[T],
    same: impl Fn(&mut BodyEq<'_, '_>, &T, &T) -> bool,
) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left_item, right_item)| same(eq, left_item, right_item))
}

/// Returns whether two optional items are both absent or equal.
fn same_option<T>(
    eq: &mut BodyEq<'_, '_>,
    left: Option<&T>,
    right: Option<&T>,
    same: impl Fn(&mut BodyEq<'_, '_>, &T, &T) -> bool,
) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left_item), Some(right_item)) => same(eq, left_item, right_item),
        _ => false,
    }
}

/// Returns whether two HIR nodes have equal generic arguments, if any.
fn same_node_args(eq: &BodyEq<'_, '_>, left: HirId, right: HirId) -> bool {
    let typeck = eq.cx.typeck_results();
    match (typeck.node_args_opt(left), typeck.node_args_opt(right)) {
        (Some(left_args), Some(right_args)) => {
            left_args.len() == right_args.len()
                && left_args
                    .iter()
                    .zip(right_args.iter())
                    .all(|(left_arg, right_arg)| same_arg(left_arg, right_arg))
        }
        (left_args, right_args) => left_args.is_none() && right_args.is_none(),
    }
}

/// Returns whether two HIR nodes have corresponding recorded types.
fn same_node_type(eq: &BodyEq<'_, '_>, left: HirId, right: HirId) -> bool {
    let typeck = eq.cx.typeck_results();
    typeck
        .node_type_opt(left)
        .zip(typeck.node_type_opt(right))
        .is_some_and(|(left_ty, right_ty)| same_arg(left_ty.into(), right_ty.into()))
}

/// Returns whether two generic arguments are equal up to closure identity.
fn same_arg<'tcx>(left: GenericArg<'tcx>, right: GenericArg<'tcx>) -> bool {
    // Walk both type trees in preorder. A pair of closure types may differ only by
    // definition; the walk still compares their signatures and captures. The walks
    // have equal length whenever every visited pair matches, because each matching
    // pair has the same shape and closure arguments have a fixed layout.
    left.walk()
        .zip(right.walk())
        .all(|(left_part, right_part)| {
            left_part == right_part
                || matches!(
                    (left_part.kind(), right_part.kind()),
                    (GenericArgKind::Type(left_ty), GenericArgKind::Type(right_ty))
                        if matches!(left_ty.kind(), ty::Closure(..))
                            && matches!(right_ty.kind(), ty::Closure(..))
                )
        })
}
