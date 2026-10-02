#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "diagnostic builders return themselves, and unsupported rustc variants stay outside this loop shape"
)]

//! A lint for loops that can use `Extend::extend`.
//!
//! It recognizes a standard `for` loop whose body pushes each source item into
//! a `Vec` or `VecDeque` named by a local place. Resolved inherent methods,
//! binding identity, and HIR control-flow screening keep the recommendation
//! limited to loops that have a direct `Extend::extend` equivalent.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Block, Expr, ExprKind, HirId, Node, StmtKind, UnOp,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{
    self,
    adjustment::{Adjust, DerefAdjustKind},
};
use rustc_span::{Span, Symbol, symbol::sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_EXTEND_LOOP,
    Warn,
    "collection insertion loop can use `Extend::extend`",
    ManualExtendLoop
}

/// Diagnostic help for loops without an exact rewrite.
const HELP: &str = "use `Extend::extend`, with `map` when the inserted value is transformed";
/// Diagnostic help when an unchanged source may have custom iteration behavior.
const UNKNOWN_SOURCE_HELP: &str =
    "use `Extend::extend` only when the source's `size_hint` has no side effects";

impl<'tcx> LateLintPass<'tcx> for ManualExtendLoop {
    /// Checks a standard loop for one resolved sequence insertion.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(candidate) = extend_candidate(cx, expr) else {
            return;
        };
        // Offer the exact rewrite only when the pushed value is the unchanged loop item.
        cx.emit_span_lint(
            MANUAL_EXTEND_LOOP,
            candidate.span,
            DiagDecorator(|diag| {
                let _ = diag.primary_message("collection insertion loop can use `Extend::extend`");
                match candidate.suggestion {
                    Some(suggestion) => {
                        let _ = diag.span_suggestion(
                            candidate.span,
                            "use `Extend::extend`",
                            suggestion,
                            Applicability::MachineApplicable,
                        );
                    }
                    None => {
                        let _ = diag.help(candidate.help);
                    }
                }
            }),
        );
    }
}

/// One loop that can use `Extend::extend`.
struct Candidate {
    /// User-written span of the complete loop.
    span: Span,
    /// Exact replacement when the loop pushes its unchanged item.
    suggestion: Option<String>,
    /// Help text for loops that do not have an exact replacement.
    help: &'static str,
}

/// Returns a candidate when the loop body is one standard sequence insertion.
fn extend_candidate<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Candidate> {
    // Inspect the inner loop match once, outside macro-generated loop bodies.
    let loop_info = (!matches!(expr.kind, ExprKind::DropTemps(_)))
        .then(|| support::for_loop(cx, expr))
        .flatten()
        .filter(|loop_info| !loop_info.body.span.from_expansion())?;
    let (receiver, argument) = sequence_insertion(cx, loop_info.body, loop_info.source)?;
    // Offer the exact rewrite only for unchanged items from a known standard source.
    let is_unchanged = is_unchanged_item(cx, &loop_info, argument);
    let is_safe_source = is_known_standard_source(cx, loop_info.source);
    let suggestion = (is_unchanged && is_safe_source)
        .then(|| exact_rewrite(cx, expr, &loop_info, receiver))
        .flatten();
    let help = if is_safe_source {
        HELP
    } else {
        UNKNOWN_SOURCE_HELP
    };
    Some(Candidate {
        span: loop_info.span.source_callsite(),
        suggestion,
        help,
    })
}

/// Returns the receiver and argument of a body that only pushes into a local sequence.
fn sequence_insertion<'tcx>(
    cx: &LateContext<'tcx>,
    body: &'tcx Block<'tcx>,
    source: &'tcx Expr<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, &'tcx Expr<'tcx>)> {
    // Require one resolved `Vec::push` or `VecDeque::push_back` call.
    let action = support::peel_drop_temps(support::block_only_expr(body)?);
    let ExprKind::MethodCall(_, receiver, [argument], _) = action.kind else {
        return None;
    };
    let is_insertion =
        support::standard_collection_method(cx, action).is_some_and(|(collection, method)| {
            matches!(
                (collection.as_str(), method.as_str()),
                ("Vec", "push") | ("VecDeque", "push_back")
            )
        });
    // Moving the insertions into one call must not skip an exit or reorder a target read.
    // The source is evaluated while `extend` holds the target borrow, so it cannot read the target.
    let is_independent = place_root(cx, receiver)
        .is_some_and(|root| is_independent_source(cx, argument, source, root));
    let has_exit = support::contains_control_flow(action);
    (is_insertion && is_independent && !has_exit).then_some((receiver, argument))
}

/// Returns whether the insertion arguments avoid reads from the target root.
fn is_independent_source<'tcx>(
    cx: &LateContext<'tcx>,
    argument: &'tcx Expr<'tcx>,
    source: &'tcx Expr<'tcx>,
    root: HirId,
) -> bool {
    let argument_is_independent = !contains_local(cx, argument, root);
    let source_is_independent = !contains_local(cx, source, root);
    argument_is_independent && source_is_independent
}

/// Returns whether the pushed value is the loop binding without a coercion.
fn is_unchanged_item(
    cx: &LateContext<'_>,
    loop_info: &support::ForLoop<'_>,
    argument: &Expr<'_>,
) -> bool {
    // A coercion would change the item type that `extend` receives.
    let typeck = cx.typeck_results();
    let item = support::simple_binding(loop_info.pat);
    let is_item = item.is_some() && support::local_binding(cx, argument) == item;
    let is_uncoerced = typeck.expr_ty_adjusted(argument) == typeck.expr_ty(argument);
    is_item && is_uncoerced
}

/// Returns whether `for` uses a source with standard, side-effect-free iteration metadata.
fn is_known_standard_source(cx: &LateContext<'_>, source: &Expr<'_>) -> bool {
    fn is_standard_source_type(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
        match ty.kind() {
            ty::Adt(adt, _) => {
                cx.tcx.is_diagnostic_item(sym::Vec, adt.did())
                    || cx
                        .tcx
                        .is_diagnostic_item(Symbol::intern("VecDeque"), adt.did())
            }
            ty::Array(..) | ty::Slice(..) => true,
            ty::Ref(_, inner, _) => is_standard_source_type(cx, *inner),
            _ => false,
        }
    }

    is_standard_source_type(cx, cx.typeck_results().expr_ty(source))
}

/// Renders `target.extend(source)` with the terminator its parent context needs.
fn exact_rewrite(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    loop_info: &support::ForLoop<'_>,
    receiver: &Expr<'_>,
) -> Option<String> {
    // Reuse the user's receiver and source text so the rewrite keeps their evaluation.
    let terminator = validate_statement_terminator(cx, expr)?;
    let target = support::snippet(cx, receiver.span)?;
    let source = support::snippet(cx, loop_info.source.span)?;
    Some(format!("{target}.extend({source}){terminator}"))
}

/// Returns the local binding at the root of a safe field or dereference place.
///
/// Calls and indexing are rejected because the loop evaluates them once per item. Custom
/// dereference adjustments are rejected because `extend` evaluates the receiver once.
fn place_root(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    if !is_builtin_collection_type(cx, expr) {
        return None;
    }
    place_root_shape(cx, expr)
}

/// Returns the root of a place after checking every implicit adjustment in its path.
fn place_root_shape(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    if !has_only_builtin_adjustments(cx, expr) {
        return None;
    }

    match expr.kind {
        ExprKind::Field(base, _) => place_root_shape(cx, base),
        ExprKind::Unary(UnOp::Deref, base) if is_builtin_reference(cx, base) => {
            place_root_shape(cx, base)
        }
        _ => support::local_binding(cx, expr),
    }
}

/// Returns whether an expression has a `Vec`, `VecDeque`, or reference-to-one type.
fn is_builtin_collection_type(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    fn is_collection(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> bool {
        match ty.kind() {
            ty::Adt(adt, _) => {
                cx.tcx.is_diagnostic_item(sym::Vec, adt.did())
                    || cx
                        .tcx
                        .is_diagnostic_item(Symbol::intern("VecDeque"), adt.did())
            }
            ty::Ref(_, inner, _) => is_collection(cx, *inner),
            _ => false,
        }
    }

    is_collection(cx, cx.typeck_results().expr_ty(expr))
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

/// Returns whether an expression is a reference whose dereference is built in.
fn is_builtin_reference(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(cx.typeck_results().expr_ty(expr).kind(), ty::Ref(..))
}

/// Returns the text that must follow the replacement for its parent context.
///
/// A loop used as an expression statement needs a semicolon. A loop in a block
/// tail or a `;` statement needs none. Other parents, such as a match arm, keep
/// help-only output.
fn validate_statement_terminator(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<&'static str> {
    // Skip the generated temporary wrapper around the loop match.
    cx.tcx
        .hir_parent_iter(expr.hir_id)
        .find(|(_, node)| {
            !matches!(
                node,
                Node::Expr(Expr {
                    kind: ExprKind::DropTemps(_),
                    ..
                })
            )
        })
        .and_then(|(_, node)| match node {
            Node::Stmt(stmt) if matches!(stmt.kind, StmtKind::Expr(_)) => Some(";"),
            Node::Stmt(_) | Node::Block(_) => Some(""),
            _ => None,
        })
}

/// Returns whether an expression, including nested closure bodies, reads one local.
fn contains_local<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, id: HirId) -> bool {
    let mut visitor = LocalVisitor {
        cx,
        id,
        is_found: false,
    };
    visitor.visit_expr(expr);
    visitor.is_found
}

/// Finds a path that resolves to one local binding.
struct LocalVisitor<'cx, 'tcx> {
    /// Compiler context used for path resolution and closure bodies.
    cx: &'cx LateContext<'tcx>,
    /// Local binding to find.
    id: HirId,
    /// Whether the binding was found.
    is_found: bool,
}

impl<'tcx> Visitor<'tcx> for LocalVisitor<'_, 'tcx> {
    /// Visits paths and enters closure bodies, which can capture the binding.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        self.is_found |= support::local_binding(self.cx, expr) == Some(self.id);
        if let ExprKind::Closure(closure) = expr.kind {
            self.visit_expr(self.cx.tcx.hir_body(closure.body).value);
        }
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
