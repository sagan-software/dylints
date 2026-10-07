#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores unmatched rustc syntax variants"
)]

//! A lint for index loops that can use `slice::windows`.
//!
//! It recovers a `for index in 0..values.len().saturating_sub(1)` loop from
//! HIR, resolves `len` and `saturating_sub` to the standard methods, and
//! requires the body to use the index only in `values[index]` and
//! `values[index + 1]`. Under those conditions the loop rewrites exactly to
//! `for window in values.windows(2)` with `window[0]` and `window[1]`.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_ast::LitKind;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    BinOpKind, BindingMode, Expr, ExprKind, HirId, LangItem, Mutability, Node, Pat, PatKind,
    StructTailExpr, UnOp,
    def::Res,
    intravisit::{Visitor, walk_expr, walk_pat},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{
    hir::nested_filter,
    ty::{
        self, TyCtxt,
        adjustment::{Adjust, DerefAdjustKind},
    },
};
use rustc_span::{Ident, Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ADJACENT_WINDOW_LOOP,
    Warn,
    "adjacent index loop can use `slice::windows(2)`",
    ManualAdjacentWindowLoop
}

/// Name given to the window binding in the suggested loop.
const WINDOW: &str = "window";

impl<'tcx> LateLintPass<'tcx> for ManualAdjacentWindowLoop {
    /// Checks a standard loop for the non-panicking adjacent-pair range.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(rewrite) = adjacent_loop(cx, expr) else {
            return;
        };
        cx.emit_span_lint(
            MANUAL_ADJACENT_WINDOW_LOOP,
            rewrite.loop_span,
            DiagDecorator(|diag| {
                let help = "use `windows(2)` and read the two elements from each window";
                let diag = diag.primary_message("adjacent index loop can use `slice::windows(2)`");
                // Rewrite the range, the binding, and both index expressions together.
                let slice_name = rewrite.slice_name;
                let _configured_suggestion = diag.multipart_suggestion(
                    help,
                    vec![
                        (rewrite.pattern, WINDOW.to_owned()),
                        (rewrite.source, format!("{slice_name}.windows(2)")),
                        (rewrite.current, format!("{WINDOW}[0]")),
                        (rewrite.next, format!("{WINDOW}[1]")),
                    ],
                    Applicability::MachineApplicable,
                );
            }),
        );
    }
}

/// The source spans that the `windows(2)` rewrite replaces.
struct Rewrite {
    /// Span of the complete loop.
    loop_span: Span,
    /// Span of the loop's index binding.
    pattern: Span,
    /// Span of the `0..values.len().saturating_sub(1)` range.
    source: Span,
    /// Name of the slice binding.
    slice_name: Ident,
    /// Span of `values[index]`.
    current: Span,
    /// Span of `values[index + 1]`.
    next: Span,
}

/// Returns the rewrite for a loop with the exact adjacent-index shape.
fn adjacent_loop<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Rewrite> {
    // Recover the user loop and its plain index binding.
    let loop_info =
        support::for_loop(cx, expr).filter(|loop_info| !loop_info.body.span.from_expansion())?;
    let pattern = loop_info.pat;
    let index = plain_binding(pattern)?;

    // Resolve the range to `0..slice.len().saturating_sub(1)` over an immutable local.
    let (slice, slice_name) = adjacent_range(cx, loop_info.source)?;

    // Require the index to appear only in `slice[index]` and `slice[index + 1]`.
    let mut uses = IndexUses {
        cx,
        slice,
        index,
        binding_uses: 0,
        slice_indexes: 0,
        current: None,
        next: None,
        uses_window_name: false,
    };
    uses.visit_block(loop_info.body);
    uses.adjacent_spans().map(|(current, next)| Rewrite {
        loop_span: loop_info.span,
        pattern: pattern.span,
        source: loop_info.source.span,
        slice_name,
        current,
        next,
    })
}

/// Returns the binding of a plain `name` pattern without `mut` or `ref`.
const fn plain_binding(pattern: &Pat<'_>) -> Option<HirId> {
    match pattern.kind {
        PatKind::Binding(BindingMode::NONE, index, _, None) => Some(index),
        _ => None,
    }
}

/// Returns the slice binding of a `0..slice.len().saturating_sub(1)` range.
fn adjacent_range<'tcx>(
    cx: &LateContext<'tcx>,
    source: &'tcx Expr<'tcx>,
) -> Option<(HirId, Ident)> {
    // The range is `0..end`, and the end is `slice.len().saturating_sub(1)`.
    let end = zero_based_range_end(cx, source)?;
    let slice_expr = saturating_len_receiver(cx, end)?;
    immutable_local(cx, slice_expr)
}

/// Returns `end` from a `Range { start: 0, end }` value.
fn zero_based_range_end<'tcx>(
    cx: &LateContext<'tcx>,
    source: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Match the lowered `Range { start, end }` struct literal.
    let ExprKind::Struct(_, [start, end], StructTailExpr::None) = source.kind else {
        return None;
    };
    let is_range = matches!(
        cx.typeck_results().expr_ty(source).kind(),
        ty::Adt(range, _) if cx.tcx.is_lang_item(range.did(), LangItem::Range)
    );
    let is_ordered = start.ident.name == sym::start && end.ident.name == sym::end;
    (is_range && is_ordered && is_int_literal(start.expr, 0)).then_some(end.expr)
}

/// Returns `receiver` from `receiver.len().saturating_sub(1)`.
fn saturating_len_receiver<'tcx>(
    cx: &LateContext<'tcx>,
    end: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Match both calls, then resolve them to the standard methods.
    let ExprKind::MethodCall(_, len_call, [one], _) = end.kind else {
        return None;
    };
    let ExprKind::MethodCall(_, receiver, [], _) = len_call.kind else {
        return None;
    };
    // Validate both method owners before replacing index arithmetic.
    let is_sub = matches_owner(cx, end, "saturating_sub", |ty| {
        matches!(ty.kind(), ty::Uint(ty::UintTy::Usize))
    });
    let is_len = matches_owner(cx, len_call, "len", |ty| is_slice_or_vec(cx, ty));
    (is_int_literal(one, 1) && is_sub && is_len).then_some(receiver)
}

/// Returns whether a type is a slice or the standard `Vec`.
fn is_slice_or_vec(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
    match ty.kind() {
        ty::Slice(_) => true,
        ty::Adt(adt, _) => cx.tcx.is_diagnostic_item(sym::Vec, adt.did()),
        _ => false,
    }
}

/// Returns whether a call resolves to a named inherent method of an owner.
fn matches_owner(
    cx: &LateContext<'_>,
    call: &Expr<'_>,
    name: &str,
    owner: impl Fn(ty::Ty<'_>) -> bool,
) -> bool {
    cx.typeck_results()
        .type_dependent_def_id(call.hir_id)
        .filter(|method| cx.tcx.item_name(*method).as_str() == name)
        .and_then(|method| cx.tcx.inherent_impl_of_assoc(method))
        .is_some_and(|impl_id| {
            owner(
                cx.tcx
                    .type_of(impl_id)
                    .instantiate_identity()
                    .skip_normalization(),
            )
        })
}

/// Returns a local binding that cannot be mutated through its name or type.
fn immutable_local(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<(HirId, Ident)> {
    // Keep the receiver on built-in slices, arrays, and vectors with no overloaded adjustment.
    if !is_builtin_sequence_receiver(cx, expr) {
        return None;
    }

    // Resolve the receiver, including explicit dereferences, to a local binding pattern.
    let id = receiver_local_id(cx, expr)?;
    let Node::Pat(Pat {
        kind: PatKind::Binding(BindingMode(_, Mutability::Not), _, name, None),
        ..
    }) = cx.tcx.hir_node(id)
    else {
        return None;
    };

    // A mutable binding or `&mut` slice could be written through an index in the body.
    let mutable_ref = matches!(
        cx.typeck_results().node_type(id).kind(),
        ty::Ref(_, _, Mutability::Mut)
    );
    (!mutable_ref).then_some((id, *name))
}

/// Returns whether a receiver is a built-in sequence with only compiler adjustments.
fn is_builtin_sequence_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_builtin_sequence_type(cx, cx.typeck_results().expr_ty(expr))
        && has_only_builtin_adjustments(cx, expr)
}

/// Returns whether a type is a slice, array, vector, or immutable reference to one.
fn is_builtin_sequence_type(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
    match ty.kind() {
        ty::Slice(_) | ty::Array(..) => true,
        ty::Adt(adt, _) => cx.tcx.is_diagnostic_item(sym::Vec, adt.did()),
        ty::Ref(_, inner, Mutability::Not) => is_builtin_sequence_type(cx, *inner),
        _ => false,
    }
}

/// Returns whether an expression's implicit adjustments use only built-in operations.
fn has_only_builtin_adjustments(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.typeck_results()
        .expr_adjustments(expr)
        .iter()
        .all(|adjustment| {
            matches!(
                adjustment.kind,
                Adjust::Deref(DerefAdjustKind::Builtin) | Adjust::Borrow(_) | Adjust::Pointer(_)
            )
        })
}

/// Returns whether an expression is an immutable built-in reference.
fn is_builtin_reference(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(
        cx.typeck_results().expr_ty(expr).kind(),
        ty::Ref(_, _, Mutability::Not)
    )
}

/// Returns the local binding reached through explicit built-in dereferences.
fn receiver_local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    match expr.kind {
        ExprKind::Unary(UnOp::Deref, base) if is_builtin_reference(cx, base) => {
            receiver_local_id(cx, base)
        }
        _ => local_id(cx, expr),
    }
}

/// Returns the local binding resolved by a path expression.
fn local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    match expr.kind {
        ExprKind::Path(ref qpath) => match cx.qpath_res(qpath, expr.hir_id) {
            Res::Local(id) => Some(id),
            _ => None,
        },
        _ => None,
    }
}

/// Returns whether an expression is an integer literal with this value.
fn is_int_literal(expr: &Expr<'_>, value: u128) -> bool {
    matches!(
        expr.kind,
        ExprKind::Lit(lit) if matches!(lit.node, LitKind::Int(found, _) if found.get() == value)
    )
}

/// Records how a loop body uses the index and the slice.
struct IndexUses<'cx, 'tcx> {
    /// Compiler context used for resolution.
    cx: &'cx LateContext<'tcx>,
    /// Slice binding from the loop range.
    slice: HirId,
    /// Index binding from the loop pattern.
    index: HirId,
    /// Number of path uses of the index binding.
    binding_uses: usize,
    /// Number of index expressions on the slice binding.
    slice_indexes: usize,
    /// Span of a user-written `slice[index]`.
    current: Option<Span>,
    /// Span of a user-written `slice[index + 1]`.
    next: Option<Span>,
    /// Whether any path or binding in the body already uses the window binding name.
    uses_window_name: bool,
}

impl IndexUses<'_, '_> {
    /// Returns whether an expression is a path to one local binding.
    fn is_local(&self, expr: &Expr<'_>, id: HirId) -> bool {
        local_id(self.cx, expr) == Some(id)
    }

    /// Returns whether an index base resolves to the same safe sequence local.
    fn is_slice_receiver(&self, expr: &Expr<'_>) -> bool {
        immutable_local(self.cx, expr).is_some_and(|(id, _)| id == self.slice)
    }

    /// Returns both index spans when the index has no other use.
    fn adjacent_spans(&self) -> Option<(Span, Span)> {
        let is_exact = self.binding_uses == 2 && self.slice_indexes == 2 && !self.uses_window_name;
        self.current.zip(self.next).filter(|_| is_exact)
    }
}

impl<'tcx> Visitor<'tcx> for IndexUses<'_, 'tcx> {
    /// Closures in the body can also use the index.
    type NestedFilter = nested_filter::OnlyBodies;

    /// Returns the type context used to enter closure bodies.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Records bindings that would shadow the window binding.
    fn visit_pat(&mut self, pat: &'tcx Pat<'tcx>) {
        if let PatKind::Binding(_, _, ident, _) = pat.kind {
            self.uses_window_name |= ident.name.as_str() == WINDOW;
        }
        walk_pat(self, pat);
    }

    /// Records index uses, slice indexes, and the two adjacent index forms.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // A path can use the index or collide with the window name; an index on
        // the slice can be one of the two adjacent reads.
        match expr.kind {
            ExprKind::Path(rustc_hir::QPath::Resolved(_, path)) => {
                self.uses_window_name |= path
                    .segments
                    .iter()
                    .any(|segment| segment.ident.name.as_str() == WINDOW);
                self.binding_uses += usize::from(self.is_local(expr, self.index));
            }
            ExprKind::Index(base, position, _) if self.is_slice_receiver(base) => {
                // Keep only user-written spans so the replacement lands in the source.
                self.slice_indexes += 1;
                let user_span = (!expr.span.from_expansion()).then_some(expr.span);
                if self.is_local(position, self.index) {
                    self.current = user_span;
                } else if let ExprKind::Binary(op, left, right) = position.kind
                    && op.node == BinOpKind::Add
                    && self.is_local(left, self.index)
                    && is_int_literal(right, 1)
                {
                    self.next = user_span;
                }
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
