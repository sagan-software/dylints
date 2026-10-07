#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc variants"
)]

//! A lint to check for one-use predicate bindings.
//!
//! It identifies a local binding that stores one predicate and is consumed once
//! as the condition of the `if` that follows it, then recommends inlining the
//! expression. Uses are counted by resolved binding identity through every
//! nested expression and closure body in the rest of the block.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    BinOpKind, BindingMode, Block, ByRef, Expr, ExprKind, HirId, Mutability, PatKind, QPath, Stmt,
    StmtKind, UnOp,
    def::Res,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::{BytePos, Span, Symbol};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub ONE_USE_PREDICATE_BINDING,
    Warn,
    "one-use predicate binding could be inlined into the branch condition",
    OneUsePredicateBinding
}

impl<'tcx> LateLintPass<'tcx> for OneUsePredicateBinding {
    /// Checks each `let` statement against the `if` that follows it.
    fn check_block(&mut self, cx: &LateContext<'tcx>, block: &'tcx Block<'tcx>) {
        // Keep the lint local so the suggestion cannot cross intervening side effects.
        (0..block.stmts.len())
            .filter_map(|index| one_use_predicate_binding(cx, block, index))
            .for_each(|offense| emit(cx, &offense));
    }
}

/// One reported binding and the data needed for its rewrite.
struct Offense<'tcx> {
    /// Span of the whole `let` statement.
    stmt_span: Span,
    /// Span of the `let` statement plus the whitespace up to the `if`.
    removal_span: Span,
    /// The binding's initializer.
    init: &'tcx Expr<'tcx>,
    /// The binding path inside the `if` condition.
    use_expr: &'tcx Expr<'tcx>,
    /// Whether the condition negates the binding with `!`.
    is_negated: bool,
}

/// Returns the offense for the statement at `index`, if it is a one-use predicate binding.
fn one_use_predicate_binding<'tcx>(
    cx: &LateContext<'tcx>,
    block: &'tcx Block<'tcx>,
    index: usize,
) -> Option<Offense<'tcx>> {
    // Pair the predicate `let` with a following `if` whose condition reads the binding.
    let stmt = block.stmts.get(index)?;
    predicate_local(cx, stmt)
        .zip(following_if(block, index))
        .and_then(|((hir_id, init), (next, condition))| {
            condition_binding(condition, hir_id)
                .map(|(use_expr, is_negated)| (hir_id, init, next, use_expr, is_negated))
        })
        // The condition must be the only use in the rest of the block.
        .filter(|(hir_id, ..)| count_uses_after(cx.tcx, block, index + 1, *hir_id) == 1)
        .map(|(_, init, next, use_expr, is_negated)| Offense {
            stmt_span: stmt.span,
            removal_span: removal_span(cx, stmt.span, next.span),
            init,
            use_expr,
            is_negated,
        })
}

/// Returns the identity and initializer of a user-written generic predicate binding.
fn predicate_local<'tcx>(
    cx: &LateContext<'tcx>,
    stmt: &'tcx Stmt<'tcx>,
) -> Option<(HirId, &'tcx Expr<'tcx>)> {
    // Require an immutable local binding before validating its predicate initializer.
    let StmtKind::Let(local) = stmt.kind else {
        return None;
    };
    let PatKind::Binding(BindingMode(ByRef::No, Mutability::Not), hir_id, ident, None) =
        local.pat.kind
    else {
        return None;
    };

    // Use the typed pattern so annotated non-`bool` lookalikes stay quiet.
    let is_bool = matches!(cx.typeck_results().pat_ty(local.pat).kind(), ty::Bool);
    local
        .init
        .filter(|init| {
            is_bool
                && local.els.is_none()
                && !local.span.from_expansion()
                && is_predicate_initializer(init)
                && is_generic_predicate_name(ident.name)
        })
        .map(|init| (hir_id, init))
}

/// Returns the `if` expression after statement `index` and its condition.
fn following_if<'tcx>(
    block: &'tcx Block<'tcx>,
    index: usize,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    // The `if` is either the next statement or the block's tail expression.
    let next = block.stmts.get(index + 1).map_or(block.expr, stmt_expr)?;
    match next.kind {
        ExprKind::If(condition, _, _) => Some((next, condition)),
        _ => None,
    }
}

/// Returns the binding path when the condition is the binding or its negation.
fn condition_binding<'tcx>(
    condition: &'tcx Expr<'tcx>,
    hir_id: HirId,
) -> Option<(&'tcx Expr<'tcx>, bool)> {
    match condition.kind {
        _ if is_local(condition, hir_id) => Some((condition, false)),
        ExprKind::Unary(UnOp::Not, inner) if is_local(inner, hir_id) => Some((inner, true)),
        _ => None,
    }
}

/// Returns whether an expression is a path to the given local binding.
fn is_local(expr: &Expr<'_>, hir_id: HirId) -> bool {
    matches!(
        expr.kind,
        ExprKind::Path(QPath::Resolved(None, path)) if path.res == Res::Local(hir_id)
    )
}

/// Returns whether an initializer has a predicate shape.
fn is_predicate_initializer(expr: &Expr<'_>) -> bool {
    match expr.kind {
        ExprKind::Binary(op, lhs, rhs) => match op.node {
            BinOpKind::Eq
            | BinOpKind::Lt
            | BinOpKind::Le
            | BinOpKind::Ne
            | BinOpKind::Ge
            | BinOpKind::Gt => true,
            BinOpKind::And | BinOpKind::Or => {
                is_predicate_initializer(lhs) || is_predicate_initializer(rhs)
            }
            _ => false,
        },
        ExprKind::MethodCall(segment, ..) => is_predicate_function_name(segment.ident.name),
        ExprKind::Call(callee, _) => {
            path_last_segment_name(callee).is_some_and(is_predicate_function_name)
        }
        ExprKind::Unary(UnOp::Not, inner) => is_predicate_initializer(inner),
        _ => false,
    }
}

/// Returns whether a function name reads as a predicate.
fn is_predicate_function_name(name: Symbol) -> bool {
    let name = name.as_str();

    name.starts_with("is_")
        || name.starts_with("has_")
        || name.starts_with("can_")
        || name.starts_with("should_")
        || matches!(
            name,
            "contains"
                | "matches"
                | "starts_with"
                | "ends_with"
                | "eq_ignore_ascii_case"
                | "is_ascii"
        )
}

/// Returns whether a binding name is a predicate prefix followed only by generic words.
fn is_generic_predicate_name(name: Symbol) -> bool {
    // Split the identifier into vocabulary tokens before checking its predicate prefix.
    let name = name.as_str();
    let words = name
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    match words.as_slice() {
        [
            "is" | "has" | "have" | "can" | "should" | "contains" | "matches",
            tail @ ..,
        ] => !tail.is_empty() && tail.iter().all(|word| is_generic_predicate_word(word)),
        _ => false,
    }
}

/// Returns whether a word adds no domain meaning to a predicate name.
fn is_generic_predicate_word(word: &str) -> bool {
    matches!(
        word,
        "empty"
            | "not"
            | "non"
            | "some"
            | "none"
            | "ok"
            | "err"
            | "valid"
            | "invalid"
            | "ready"
            | "present"
            | "missing"
            | "found"
            | "available"
            | "enabled"
            | "disabled"
            | "positive"
            | "negative"
            | "zero"
            | "item"
            | "items"
            | "value"
            | "values"
            | "result"
            | "results"
            | "data"
            | "match"
            | "matches"
            | "remaining"
    )
}

/// Counts resolved uses of a binding in the block from statement `start` onward.
fn count_uses_after<'tcx>(
    tcx: TyCtxt<'tcx>,
    block: &'tcx Block<'tcx>,
    start: usize,
    hir_id: HirId,
) -> usize {
    // Earlier statements cannot see the binding, so start at `start`.
    let mut counter = UseCounter {
        tcx,
        hir_id,
        count: 0,
    };

    // Visit the remaining statements, then the tail expression.
    for stmt in block.stmts.get(start..).unwrap_or_default() {
        counter.visit_stmt(stmt);
    }
    if let Some(expr) = block.expr {
        counter.visit_expr(expr);
    }
    counter.count
}

/// Visitor that counts resolved paths to one local, including inside closures.
struct UseCounter<'tcx> {
    /// Type context used to enter closure bodies.
    tcx: TyCtxt<'tcx>,
    /// The counted binding.
    hir_id: HirId,
    /// Number of uses found so far.
    count: usize,
}

impl<'tcx> Visitor<'tcx> for UseCounter<'tcx> {
    /// Counts one path use and descends into closure bodies.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Count a resolved path to the binding.
        if is_local(expr, self.hir_id) {
            self.count += 1;
        }

        // The default walk does not enter closure bodies, so visit them explicitly.
        if let ExprKind::Closure(closure) = expr.kind {
            self.visit_body(self.tcx.hir_body(closure.body));
        }
        walk_expr(self, expr);
    }
}

/// Returns the expression carried by an expression statement.
const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Returns the last path segment of a callee.
fn path_last_segment_name(expr: &Expr<'_>) -> Option<Symbol> {
    let ExprKind::Path(qpath) = expr.kind else {
        return None;
    };

    match qpath {
        QPath::Resolved(_, path) => path.segments.last().map(|segment| segment.ident.name),
        QPath::TypeRelative(_, segment) => Some(segment.ident.name),
    }
}

/// Returns the `let` span extended over the whitespace that follows it.
fn removal_span(cx: &LateContext<'_>, stmt_span: Span, next_span: Span) -> Span {
    // Stop at the next comment or token so it moves into the `let` statement's position.
    let gap = stmt_span.between(next_span);
    let whitespace_len = cx
        .sess()
        .source_map()
        .span_to_snippet(gap)
        .ok()
        .and_then(|text| u32::try_from(text.len() - text.trim_start().len()).ok())
        .unwrap_or(0);
    stmt_span.with_hi(gap.lo() + BytePos(whitespace_len))
}

/// Returns the replacement for the binding path, or `None` when no exact rewrite exists.
fn replacement(cx: &LateContext<'_>, offense: &Offense<'_>) -> Option<String> {
    // Macro-produced code has no user-written text to move.
    let same_context = offense.init.span.eq_ctxt(offense.stmt_span)
        && offense.use_expr.span.eq_ctxt(offense.stmt_span)
        && !offense.stmt_span.from_expansion()
        && !offense.use_expr.span.from_expansion();
    if !same_context {
        return None;
    }
    let text = cx
        .sess()
        .source_map()
        .span_to_snippet(offense.init.span)
        .ok()?;

    // A struct literal cannot appear unparenthesized in an `if` condition, and `!` binds tighter
    // than a binary operator.
    let needs_parens = contains_struct_literal(offense.init)
        || (offense.is_negated && matches!(offense.init.kind, ExprKind::Binary(..)));
    Some(if needs_parens {
        format!("({text})")
    } else {
        text
    })
}

/// Returns whether an expression contains a struct literal outside a closure body.
fn contains_struct_literal(expr: &Expr<'_>) -> bool {
    /// Visitor that records whether it saw a struct literal.
    struct Finder(bool);
    impl<'tcx> Visitor<'tcx> for Finder {
        /// Records struct literals.
        fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
            self.0 |= matches!(expr.kind, ExprKind::Struct(..));
            walk_expr(self, expr);
        }
    }
    let mut finder = Finder(false);
    finder.visit_expr(expr);
    finder.0
}

/// Emits the lint with a machine-applicable rewrite when one is exact.
fn emit(cx: &LateContext<'_>, offense: &Offense<'_>) {
    let replacement = replacement(cx, offense);
    let removal_span = offense.removal_span;
    let use_span = offense.use_expr.span;
    cx.emit_span_lint(
        ONE_USE_PREDICATE_BINDING,
        offense.stmt_span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message("one-use predicate binding could be inlined");
            let help = "inline the predicate expression into the branch condition unless the name captures a domain concept";
            if let Some(replacement) = replacement {
                let _ = diag.multipart_suggestion(
                    help,
                    vec![(removal_span, String::new()), (use_span, replacement)],
                    Applicability::MachineApplicable,
                );
            } else {
                let _ = diag.help(help);
            }
        }),
    );
}

/// Runs the UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
