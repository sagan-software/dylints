#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]

//! A lint to check for complicated boolean conditionals.
//!
//! It flattens source boolean chains, scores resolved terms and nested control
//! flow, and reports `if`, `while`, and match-guard conditions with complex
//! inline work. `let` initializers and assignments are skipped because they
//! already name the result. Diagnostics recommend named predicates while
//! preserving evaluation semantics.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Arm, BinOpKind, Expr, ExprKind, MatchSource,
    intravisit::{self, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{ExpnKind, MacroKind, Span};

/// Score at which one `&&` or `||` term counts as complex.
const COMPLEX_TERM_SCORE: usize = 6;
/// Score at which a whole condition, or a condition without `&&` or `||`, is
/// reported.
const COMPLEX_CHAIN_SCORE: usize = 9;
/// Maximum work counted inside one inline closure.
const CLOSURE_WORK_CAP: usize = 8;
/// Closure work that adds nothing, so a closure with one call or one comparison
/// is free.
const CLOSURE_WORK_ALLOWANCE: usize = 2;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub COMPLICATED_CONDITIONAL,
    Warn,
    "complicated boolean conditional could use named predicates",
    ComplicatedConditional
}

impl<'tcx> LateLintPass<'tcx> for ComplicatedConditional {
    /// Inspect `if` and `while` conditions.
    ///
    /// A `let` initializer or an assignment already names its boolean, which is the
    /// rewrite this lint recommends, so those contexts are not checked.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // HIR represents a while condition as an if inside the lowered loop.
        if let ExprKind::If(condition, _, _) = expr.kind {
            check_boolean(cx, condition);
        }
    }

    /// Inspect source match guards independently of their arm bodies.
    fn check_arm(&mut self, cx: &LateContext<'tcx>, arm: &'tcx Arm<'tcx>) {
        if let Some(guard) = arm.guard {
            check_boolean(cx, guard);
        }
    }
}

/// Report one complete boolean gate without rewriting evaluation.
fn check_boolean(cx: &LateContext<'_>, expression: &Expr<'_>) {
    if complicated_condition(cx, expression) {
        emit_span_lint_with_help(
            cx,
            COMPLICATED_CONDITIONAL,
            expression.span,
            "boolean expression contains complex inline work",
            "extract complex predicates into named boolean variables; preserve lazy evaluation and reevaluate loop predicates on every iteration",
        );
    }
}

/// Helper for complicated condition analysis.
fn complicated_condition(cx: &LateContext<'_>, condition: &Expr<'_>) -> bool {
    // Reject non-boolean values before traversing their potentially large initializers.
    // Exclude generated conditions because their source cannot be extracted safely.
    let is_bool = cx
        .typeck_results()
        .expr_ty(peel_drop_temps(condition))
        .is_bool();
    if !is_bool || contains_macro_expansion(condition) {
        return false;
    }

    // Flatten the top-level boolean chain before scoring individual terms.
    let mut terms = Vec::new();
    collect_boolean_terms(condition, &mut terms);
    if terms.len() < 2 {
        return complexity_score(cx, condition) >= COMPLEX_CHAIN_SCORE;
    }

    // Combine the number of complex terms with the total chain score.
    let mut complex_terms = 0;
    let mut total_score = 0;
    for term in terms {
        let score = complexity_score(cx, term);
        complex_terms += usize::from(score >= COMPLEX_TERM_SCORE);
        total_score += score;
    }

    complex_terms >= 2 || (complex_terms > 0 && total_score >= COMPLEX_CHAIN_SCORE)
}

/// Collect boolean terms used by the lint.
fn collect_boolean_terms<'tcx>(expr: &'tcx Expr<'tcx>, terms: &mut Vec<&'tcx Expr<'tcx>>) {
    // Remove compiler wrappers before recognizing boolean operators.
    let expr = peel_drop_temps(expr);
    if let ExprKind::Binary(op, lhs, rhs) = expr.kind
        && matches!(op.node, BinOpKind::And | BinOpKind::Or)
    {
        // Preserve evaluation order while recursively flattening both operands.
        collect_boolean_terms(lhs, terms);
        collect_boolean_terms(rhs, terms);
        return;
    }

    terms.push(expr);
}

/// Helper for complexity score analysis.
fn complexity_score(cx: &LateContext<'_>, expr: &Expr<'_>) -> usize {
    // Score only resolved boolean expressions after removing compiler wrappers.
    let expr = peel_drop_temps(expr);
    if !cx.typeck_results().expr_ty(expr).is_bool() {
        return 0;
    }

    expression_complexity_score(cx, expr)
}

/// Scores one resolved boolean expression after its type has been checked.
fn expression_complexity_score(cx: &LateContext<'_>, expr: &Expr<'_>) -> usize {
    // Weight control flow and call chains above direct field access.
    match expr.kind {
        ExprKind::Field(receiver, _) => field_score(cx, receiver),
        ExprKind::MethodCall(_, receiver, args, _) => {
            2 + value_chain_score(cx, receiver) + bool_args_complexity(cx, args)
        }
        ExprKind::Call(callee, args) => {
            2 + value_chain_score(cx, callee) + bool_args_complexity(cx, args)
        }
        ExprKind::Binary(op, lhs, rhs) => binary_complexity_score(cx, op.node, lhs, rhs),
        _ => remaining_expression_complexity_score(cx, expr),
    }
}

/// Scores expression variants that need nested boolean-child analysis.
fn remaining_expression_complexity_score(cx: &LateContext<'_>, expr: &Expr<'_>) -> usize {
    match expr.kind {
        ExprKind::Unary(_, inner)
        | ExprKind::Cast(inner, _)
        | ExprKind::Type(inner, _)
        | ExprKind::Use(inner, _)
        | ExprKind::AddrOf(_, _, inner) => 1 + bool_child_score(cx, inner),
        ExprKind::Block(block, _) => 2 + block.expr.map_or(0, |expr| complexity_score(cx, expr)),
        ExprKind::If(condition, then_expr, else_expr) => {
            // Score the condition and each boolean-producing branch.
            4 + complexity_score(cx, condition)
                + bool_child_score(cx, then_expr)
                + else_expr.map_or(0, |expr| bool_child_score(cx, expr))
        }
        ExprKind::Match(scrutinee, arms, _) => {
            // Include each match guard and boolean-producing arm body.
            4 + bool_child_score(cx, scrutinee)
                + arms
                    .iter()
                    .map(|arm| {
                        arm.guard.map_or(0, |guard| bool_child_score(cx, guard))
                            + bool_child_score(cx, arm.body)
                    })
                    .sum::<usize>()
        }
        ExprKind::Let(..) => 2,
        ExprKind::Closure(_) => 3,
        ExprKind::DropTemps(inner) => complexity_score(cx, inner),
        _ => 0,
    }
}

/// Scores a binary boolean expression and its child predicates.
fn binary_complexity_score(
    cx: &LateContext<'_>,
    operator: BinOpKind,
    left: &Expr<'_>,
    right: &Expr<'_>,
) -> usize {
    match operator {
        BinOpKind::And | BinOpKind::Or => complexity_score(cx, left) + complexity_score(cx, right),
        _ => 1 + comparison_operand_score(cx, left) + comparison_operand_score(cx, right),
    }
}

/// Include value-producing chains when a comparison turns them into a boolean.
fn comparison_operand_score(cx: &LateContext<'_>, operand: &Expr<'_>) -> usize {
    if cx
        .typeck_results()
        .expr_ty(peel_drop_temps(operand))
        .is_bool()
    {
        complexity_score(cx, operand)
    } else {
        value_chain_score(cx, operand)
    }
}

/// Helper for bool child score analysis.
fn bool_child_score(cx: &LateContext<'_>, expr: &Expr<'_>) -> usize {
    let expr = peel_drop_temps(expr);
    if cx.typeck_results().expr_ty(expr).is_bool() {
        complexity_score(cx, expr)
    } else {
        0
    }
}

/// Helper for bool args complexity analysis.
fn bool_args_complexity(cx: &LateContext<'_>, args: &[Expr<'_>]) -> usize {
    args.iter()
        .map(|arg| {
            if let ExprKind::Closure(closure) = arg.kind {
                // Closure bodies have separate type tables; score their source structure only.
                let body = cx.tcx.hir_body(closure.body);
                let mut visitor = ClosureWork { score: 0 };
                visitor.visit_expr(body.value);
                visitor.score.saturating_sub(CLOSURE_WORK_ALLOWANCE)
            } else {
                bool_child_score(cx, arg)
            }
        })
        .sum()
}

/// Helper for value chain score analysis.
fn value_chain_score(cx: &LateContext<'_>, expr: &Expr<'_>) -> usize {
    match peel_drop_temps(expr).kind {
        ExprKind::Field(receiver, _) => field_score(cx, receiver),
        ExprKind::MethodCall(_, receiver, args, _) => {
            2 + value_chain_score(cx, receiver) + bool_args_complexity(cx, args)
        }
        ExprKind::Call(callee, args) => {
            2 + value_chain_score(cx, callee) + bool_args_complexity(cx, args)
        }
        ExprKind::Match(scrutinee, _arms, source) => match_chain_score(cx, scrutinee, source),
        ExprKind::Index(receiver, _index, _) => 2 + value_chain_score(cx, receiver),
        ExprKind::Unary(_, inner)
        | ExprKind::Cast(inner, _)
        | ExprKind::Type(inner, _)
        | ExprKind::Use(inner, _)
        | ExprKind::AddrOf(_, _, inner) => 1 + value_chain_score(cx, inner),
        ExprKind::DropTemps(inner) => value_chain_score(cx, inner),
        _ => 0,
    }
}

/// Scores a `match` that produces a receiver or comparison operand.
///
/// rustc lowers `operand?` and `operand.await` to a match on a `Try::branch` or
/// `IntoFuture::into_future` call. Those add nothing themselves; the chain the
/// reader wrote as the operand is scored instead. A source `match` adds 3.
fn match_chain_score(cx: &LateContext<'_>, scrutinee: &Expr<'_>, source: MatchSource) -> usize {
    match (source, scrutinee.kind) {
        (MatchSource::TryDesugar(_) | MatchSource::AwaitDesugar, ExprKind::Call(_, [operand])) => {
            value_chain_score(cx, operand)
        }
        _ => 3 + value_chain_score(cx, scrutinee),
    }
}

/// Scores a field access: free on a named place such as `self.cx.tcx`, 1 plus the
/// receiver's chain score on a computed value such as `order.customer().address`.
fn field_score(cx: &LateContext<'_>, receiver: &Expr<'_>) -> usize {
    if is_named_place(receiver) {
        0
    } else {
        1 + value_chain_score(cx, receiver)
    }
}

/// Return whether an expression is a path or a chain of field accesses on a path.
fn is_named_place(expr: &Expr<'_>) -> bool {
    match peel_drop_temps(expr).kind {
        ExprKind::Path(_) => true,
        ExprKind::Field(receiver, _) => is_named_place(receiver),
        _ => false,
    }
}

/// Helper for peel drop temps analysis.
fn peel_drop_temps<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Counts visible work inside a closure without entering nested item bodies.
struct ClosureWork {
    /// Structural work units, capped at `CLOSURE_WORK_CAP`.
    score: usize,
}

impl<'tcx> Visitor<'tcx> for ClosureWork {
    /// Count calls and control flow while bounding work in large closure bodies.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Stop once the cap is reached or the code comes from a macro.
        if self.score >= CLOSURE_WORK_CAP || from_macro_expansion(expr.span) {
            return;
        }
        // Weight calls, branches, nested closures, and operators, then walk children.
        let weight = match expr.kind {
            ExprKind::Call(..) | ExprKind::MethodCall(..) => 2,
            ExprKind::If(..) | ExprKind::Match(..) | ExprKind::Loop(..) | ExprKind::Closure(..) => {
                4
            }
            ExprKind::Binary(..) | ExprKind::Assign(..) => 1,
            _ => 0,
        };
        self.score = (self.score + weight).min(CLOSURE_WORK_CAP);
        intravisit::walk_expr(self, expr);
    }

    /// Count intermediate statements as work even when their values are simple.
    fn visit_stmt(&mut self, statement: &'tcx rustc_hir::Stmt<'tcx>) {
        self.score = (self.score + 1).min(CLOSURE_WORK_CAP);
        intravisit::walk_stmt(self, statement);
    }
}

/// Return whether the source contains macro expansion.
fn contains_macro_expansion(expr: &Expr<'_>) -> bool {
    let mut visitor = MacroExpansionVisitor { is_found: false };
    visitor.visit_expr(expr);
    visitor.is_found
}

/// Tracks whether an expression subtree contains a source macro expansion.
struct MacroExpansionVisitor {
    /// Set after the first generated expression is found.
    is_found: bool,
}

impl<'tcx> Visitor<'tcx> for MacroExpansionVisitor {
    /// Stop traversing after a generated expression has been identified.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Do not revisit children after a generated expression has been found.
        if self.is_found {
            return;
        }
        // Mark the first generated expression before descending into its children.
        if from_macro_expansion(expr.span) {
            self.is_found = true;
            return;
        }
        // Walk source-authored children using rustc's forward-compatible visitor.
        intravisit::walk_expr(self, expr);
    }
}

/// Helper for from macro expansion analysis.
fn from_macro_expansion(span: Span) -> bool {
    // Walk outward through nested expansion contexts.
    let mut expn_data = span.ctxt().outer_expn_data();

    loop {
        if matches!(
            expn_data.kind,
            ExpnKind::Macro(MacroKind::Bang | MacroKind::Attr | MacroKind::Derive, _)
        ) {
            return true;
        }

        // Stop at the first source-authored call site.
        if !expn_data.call_site.from_expansion() {
            return false;
        }
        expn_data = expn_data.call_site.ctxt().outer_expn_data();
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
