#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unsupported rustc expression variants do not form direct insertion branches"
)]
#![warn(unused_extern_crates)]

//! A lint for loops that can use `Iterator::partition`.
//!
//! It recognizes two empty mutable standard collections of one type, a loop
//! whose body inserts the loop item into one collection or the other, and a
//! tuple containing the collections afterward. Constructors, insertion methods,
//! inserted items, and accumulators are all checked through resolved
//! definitions and local bindings.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    BindingMode, Block, ByRef, Expr, ExprKind, HirId, MatchSource, Mutability, Pat, PatKind, Stmt,
    StmtKind,
    def::{DefKind, Res},
    def_id::DefId,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::{
    hir::nested_filter,
    ty::{self, Ty, TyCtxt},
};
use rustc_span::{Span, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_PARTITION_LOOP,
    Warn,
    "two-way collection loop can use `Iterator::partition`",
    ManualPartitionLoop
}

/// One empty mutable collection declared before the loop.
#[derive(Clone, Copy)]
struct Accumulator<'tcx> {
    /// Binding that names the collection.
    id: HirId,
    /// Full collection type, which must match the other accumulator.
    ty: Ty<'tcx>,
    /// Standard collection definition.
    adt: DefId,
}

impl<'tcx> LateLintPass<'tcx> for ManualPartitionLoop {
    /// Checks two new collections, one loop, and the matching tuple tail.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Scan only adjacent initializer pairs followed by one loop.
        let Some(tail) = block.expr else {
            return;
        };

        // Resolve each adjacent pair of accumulators and its following loop.
        for statements in block.stmts.windows(3) {
            if let [first_stmt, second_stmt, loop_stmt] = statements
                && let Some(span) = partition_loop(cx, tail, first_stmt, second_stmt, loop_stmt)
            {
                // Report the loop after its initializers, branches, and tuple tail agree.
                support::emit(
                    cx,
                    MANUAL_PARTITION_LOOP,
                    span,
                    "two-way collection loop can use `Iterator::partition`",
                    "use `Iterator::partition` and bind its two returned collections",
                );
            }
        }
    }
}

/// Returns the loop span when the accumulators, loop, and tail form a partition.
fn partition_loop<'tcx>(
    cx: &LateContext<'tcx>,
    tail: &'tcx Expr<'tcx>,
    first_stmt: &'tcx Stmt<'tcx>,
    second_stmt: &'tcx Stmt<'tcx>,
    loop_stmt: &'tcx Stmt<'tcx>,
) -> Option<Span> {
    // Resolve both accumulators before the loop.
    let first = accumulator(cx, first_stmt)?;
    let second = accumulator(cx, second_stmt)?;

    // `partition` returns one collection type for both sides, in tuple order.
    let is_pair = first.ty == second.ty && is_tuple(cx, tail, first.id, second.id);

    // The loop must bind the whole item and insert it in each branch.
    let loop_info =
        support::stmt_expr(loop_stmt).and_then(|loop_expr| support::for_loop(cx, loop_expr))?;
    let item = item_binding(loop_info.pat)?;
    (is_pair && is_partition_body(cx, loop_info.body, item, first, second))
        .then_some(loop_info.span)
}

/// Parses an empty mutable standard collection binding.
fn accumulator<'tcx>(cx: &LateContext<'tcx>, stmt: &Stmt<'tcx>) -> Option<Accumulator<'tcx>> {
    // Require a mutable binding with one local identity.
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Mut), id, _, None) = local.pat.kind
    else {
        return None;
    };

    // Accept only standard collections that have a single-item insertion method.
    let ty = cx.typeck_results().pat_ty(local.pat);
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };
    (insertion_method(cx, adt.did()).is_some() && is_empty_constructor(cx, local.init?, adt.did()))
        .then_some(Accumulator {
            id,
            ty,
            adt: adt.did(),
        })
}

/// Standard collections by diagnostic item, with their single-item insertion method.
const INSERTION_METHODS: [(&str, &str); 4] = [
    ("Vec", "push"),
    ("VecDeque", "push_back"),
    ("HashSet", "insert"),
    ("BTreeSet", "insert"),
];

/// Returns the single-item insertion method name for a supported collection.
fn insertion_method(cx: &LateContext<'_>, adt: DefId) -> Option<&'static str> {
    let name = cx.tcx.get_diagnostic_name(adt)?;
    INSERTION_METHODS
        .iter()
        .find(|(collection, _)| name.as_str() == *collection)
        .map(|(_, method)| *method)
}

/// Returns whether an initializer calls `Default::default` or the type's `new`.
fn is_empty_constructor(cx: &LateContext<'_>, init: &Expr<'_>, adt: DefId) -> bool {
    // Resolve the zero-argument callee, including type-relative paths such as `Vec::new`.
    let ExprKind::Call(callee, []) = init.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Res::Def(DefKind::AssocFn, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return false;
    };

    // `new` must belong to the collection's own inherent `impl` block.
    cx.tcx.is_diagnostic_item(sym::default_fn, def_id)
        || (cx.tcx.item_name(def_id) == sym::new && inherent_owner(cx, def_id) == Some(adt))
}

/// Returns the ADT whose inherent `impl` block defines an associated function.
fn inherent_owner(cx: &LateContext<'_>, def_id: DefId) -> Option<DefId> {
    let impl_id = cx.tcx.inherent_impl_of_assoc(def_id)?;
    match cx
        .tcx
        .type_of(impl_id)
        .instantiate_identity()
        .skip_normalization()
        .kind()
    {
        ty::Adt(adt, _) => Some(adt.did()),
        _ => None,
    }
}

/// Returns the binding when a loop pattern binds the whole item by value.
const fn item_binding(pat: &Pat<'_>) -> Option<HirId> {
    match pat.kind {
        PatKind::Binding(BindingMode(ByRef::No, _), id, _, None) => Some(id),
        _ => None,
    }
}

/// Checks opposite branches that insert the loop item into the two accumulators.
fn is_partition_body<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    item: HirId,
    first: Accumulator<'tcx>,
    second: Accumulator<'tcx>,
) -> bool {
    // Match one `if`/`else` whose condition can become a closure body.
    let Some(body) = support::block_only_expr(block) else {
        return false;
    };
    let ExprKind::If(condition, true_branch, Some(false_branch)) = body.kind else {
        return false;
    };
    if !is_closure_safe_condition(cx, condition, &[first.id, second.id]) {
        return false;
    }

    // Resolve both branches to direct insertions of the item itself.
    is_branch_insertion(cx, true_branch, item, first)
        && is_branch_insertion(cx, false_branch, item, second)
}

/// Returns whether one branch only inserts the loop item into one accumulator.
fn is_branch_insertion<'tcx>(
    cx: &LateContext<'tcx>,
    branch: &'tcx Expr<'tcx>,
    item: HirId,
    target: Accumulator<'tcx>,
) -> bool {
    // Peel the branch block down to one method call statement.
    let ExprKind::Block(block, _) = branch.kind else {
        return false;
    };
    let Some(action) = support::block_only_expr(block) else {
        return false;
    };
    let ExprKind::MethodCall(_, receiver, [argument], _) = action.kind else {
        return false;
    };

    // Require the collection's own insertion method, target, and the unchanged item.
    cx.typeck_results()
        .type_dependent_def_id(action.hir_id)
        .zip(insertion_method(cx, target.adt))
        .is_some_and(|(method, name)| {
            cx.tcx.item_name(method).as_str() == name
                && inherent_owner(cx, method) == Some(target.adt)
        })
        && local_id(cx, receiver) == Some(target.id)
        && local_id(cx, argument) == Some(item)
}

/// Checks the ordered tuple tail.
fn is_tuple(cx: &LateContext<'_>, expr: &Expr<'_>, first: HirId, second: HirId) -> bool {
    let ExprKind::Tup([left, right]) = support::peel_drop_temps(expr).kind else {
        return false;
    };
    local_id(cx, left) == Some(first) && local_id(cx, right) == Some(second)
}

/// Returns the local binding resolved by a path expression.
fn local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    // Use HIR resolution so shadowed names remain distinct.
    let ExprKind::Path(qpath) = support::peel_drop_temps(expr).kind else {
        return None;
    };
    match cx.qpath_res(&qpath, expr.hir_id) {
        Res::Local(id) => Some(id),
        _ => None,
    }
}

/// Returns whether a condition can move into a predicate closure unchanged.
fn is_closure_safe_condition<'tcx>(
    cx: &LateContext<'tcx>,
    condition: &'tcx Expr<'tcx>,
    accumulators: &[HirId],
) -> bool {
    let mut finder = ConditionFinder {
        cx,
        accumulators,
        is_blocked: false,
    };
    finder.visit_expr(condition);
    !finder.is_blocked
}

/// Finds condition parts that a predicate closure cannot keep.
struct ConditionFinder<'cx, 'ids, 'tcx> {
    /// Compiler context used for path resolution.
    cx: &'cx LateContext<'tcx>,
    /// Accumulators that `partition` owns until it returns.
    accumulators: &'ids [HirId],
    /// Whether a blocking expression was found.
    is_blocked: bool,
}

impl<'tcx> Visitor<'tcx> for ConditionFinder<'_, '_, 'tcx> {
    /// Closures in the condition can also capture an accumulator.
    type NestedFilter = nested_filter::OnlyBodies;

    /// Returns the type context used to enter closure bodies.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Blocks accumulator reads and control flow that would leave the closure.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.is_blocked |= match expr.kind {
            ExprKind::Path(ref qpath) => matches!(
                self.cx.qpath_res(qpath, expr.hir_id),
                Res::Local(id) if self.accumulators.contains(&id)
            ),
            ExprKind::Ret(_)
            | ExprKind::Break(..)
            | ExprKind::Continue(_)
            | ExprKind::Become(_)
            | ExprKind::Yield(..)
            | ExprKind::Match(_, _, MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar) => true,
            _ => false,
        };
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
