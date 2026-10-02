#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unsupported rustc expression and pattern variants never form a chainable loop pair"
)]

//! A lint for consecutive iterator loops that can use `Iterator::chain`.
//!
//! It recovers two adjacent desugared `for` loops, requires each source to be a
//! local binding or a direct borrow of one, and compares the loop patterns and
//! bodies structurally. Uses of the first loop's bindings must correspond to
//! uses of the second loop's bindings at the same pattern position.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

mod body_eq;
#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    Block, Expr, ExprKind, QPath,
    def::Res,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

use self::body_eq::BodyEq;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CONSECUTIVE_ITERATOR_LOOPS,
    Warn,
    "consecutive iterator loops can use `Iterator::chain`",
    ConsecutiveIteratorLoops
}

impl<'tcx> LateLintPass<'tcx> for ConsecutiveIteratorLoops {
    /// Checks adjacent standard loops with local sources and equal bodies.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Check statement pairs first, then the common statement-plus-tail form.
        let statement_pairs = block.stmts.array_windows().filter_map(|[first, second]| {
            Some((support::stmt_expr(first)?, support::stmt_expr(second)?))
        });
        let tail_pair = block
            .stmts
            .last()
            .and_then(support::stmt_expr)
            .zip(block.expr);
        statement_pairs
            .chain(tail_pair)
            .for_each(|(first_expr, second_expr)| check_pair(cx, first_expr, second_expr));
    }
}

/// Checks one adjacent expression pair and emits for two compatible loops.
fn check_pair<'tcx>(
    cx: &LateContext<'tcx>,
    first_expr: &'tcx Expr<'tcx>,
    second_expr: &'tcx Expr<'tcx>,
) {
    if let Some(span) = chainable_loop_span(cx, first_expr, second_expr) {
        support::emit(
            cx,
            CONSECUTIVE_ITERATOR_LOOPS,
            span,
            "consecutive iterator loops can use `Iterator::chain`",
            "use `Iterator::chain` and apply the shared body once",
        );
    }
}

/// Returns the combined span for two loops with local sources and equal bodies.
fn chainable_loop_span<'tcx>(
    cx: &LateContext<'tcx>,
    first_expr: &'tcx Expr<'tcx>,
    second_expr: &'tcx Expr<'tcx>,
) -> Option<Span> {
    // Recover two loops whose sources and bodies allow chaining.
    let first = support::for_loop(cx, first_expr)?;
    let second = support::for_loop(cx, second_expr)?;
    let is_chainable = is_chainable_loop(cx, &first) && is_chainable_loop(cx, &second);

    // Pair the pattern bindings, then compare both bodies modulo that pairing.
    let first_pat = first.pat;
    let second_pat = second.pat;
    let mut eq = BodyEq::new(cx);
    (is_chainable && eq.same_pat(first_pat, second_pat) && eq.same_block(first.body, second.body))
        .then(|| first.span.to(second.span))
}

/// Returns whether one loop is user-written, has a local source, and never breaks.
fn is_chainable_loop(cx: &LateContext<'_>, loop_info: &support::ForLoop<'_>) -> bool {
    // The loop shell is a desugaring, so the body span tells whether a macro wrote it.
    let is_user_written = !loop_info.body.span.from_expansion();

    // A `break` leaves only its own loop, but it would leave both after chaining.
    is_user_written && is_local_source(cx, loop_info.source) && !contains_break(loop_info.body)
}

/// Returns whether a loop source is a local binding or a direct borrow of one.
fn is_local_source(cx: &LateContext<'_>, source: &Expr<'_>) -> bool {
    let place = match source.kind {
        ExprKind::AddrOf(_, _, inner) => inner,
        _ => source,
    };
    matches!(
        place.kind,
        ExprKind::Path(ref qpath @ QPath::Resolved(None, _))
            if matches!(cx.qpath_res(qpath, place.hir_id), Res::Local(_))
    )
}

/// Returns whether a loop body contains any `break` expression.
fn contains_break(body: &Block<'_>) -> bool {
    let mut finder = BreakFinder { has_break: false };
    finder.visit_block(body);
    finder.has_break
}

/// Records whether a visited body contains a `break`.
struct BreakFinder {
    /// Whether a `break` was found.
    has_break: bool,
}

impl<'tcx> Visitor<'tcx> for BreakFinder {
    /// Marks `break` expressions and continues the walk.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.has_break |= matches!(expr.kind, ExprKind::Break(..));
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
