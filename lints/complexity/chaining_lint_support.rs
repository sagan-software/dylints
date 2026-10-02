//! Shared semantic helpers for the method-chaining lint family.

#![allow(
    dead_code,
    reason = "each constituent lint uses a different subset of these shared helpers"
)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the shared lint helpers intentionally ignore diagnostic builders and unmatched rustc variants"
)]

use rustc_errors::DiagDecorator;
use rustc_hir::{Block, Expr, ExprKind, MatchSource, Stmt, StmtKind, def::Res};
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

/// The source, pattern, and body recovered from one standard `for` loop.
pub(crate) struct ForLoop<'tcx> {
    /// Span of the complete `for` loop.
    pub(crate) span: Span,
    /// Expression passed to `IntoIterator::into_iter`.
    pub(crate) source: &'tcx Expr<'tcx>,
    /// User-written loop body.
    pub(crate) body: &'tcx Block<'tcx>,
}

/// Recovers a standard `for` loop from rustc's desugared HIR.
pub(crate) fn for_loop<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<ForLoop<'tcx>> {
    // Peel the outer temporary wrapper and match rustc's standard for-loop shell.
    let expr = peel_drop_temps(expr);
    match expr.kind {
        ExprKind::Match(iter_expr, [iter_arm], MatchSource::ForLoopDesugar) => {
            Some((iter_expr, iter_arm))
        }
        _ => None,
    }
    .and_then(|(iter_expr, iter_arm)| match iter_expr.kind {
        ExprKind::Call(callee, [source]) if resolved_into_iter(cx, callee) => {
            Some((source, iter_arm))
        }
        _ => None,
    })
    // Descend through the generated loop and iterator match to the user body.
    .and_then(|(source, iter_arm)| match iter_arm.body.kind {
        ExprKind::Loop(loop_block, _, _, _) => Some((source, loop_block)),
        _ => None,
    })
    .and_then(|(source, loop_block)| {
        block_only_expr(loop_block).and_then(|expr| match expr.kind {
            ExprKind::Match(_, arms, MatchSource::ForLoopDesugar) => Some((source, arms)),
            _ => None,
        })
    })
    // Select the arm that carries the user-written block expression.
    .and_then(|(source, arms)| {
        arms.iter().find_map(|arm| match arm.body.kind {
            ExprKind::Block(body, _) => Some(ForLoop {
                span: expr.span,
                source,
                body,
            }),
            _ => None,
        })
    })
}

/// Returns the only expression in a block.
pub(crate) const fn block_only_expr<'tcx>(block: &'tcx Block<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match (block.stmts, block.expr) {
        ([stmt], None) => stmt_expr(stmt),
        ([], Some(expr)) => Some(expr),
        _ => None,
    }
}

/// Returns the expression carried by an expression statement.
pub(crate) const fn stmt_expr<'tcx>(stmt: &'tcx Stmt<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match stmt.kind {
        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(expr),
        StmtKind::Let(_) | StmtKind::Item(_) => None,
    }
}

/// Removes compiler-generated temporary wrappers.
pub(crate) fn peel_drop_temps<'tcx>(expr: &'tcx Expr<'tcx>) -> &'tcx Expr<'tcx> {
    match expr.kind {
        ExprKind::DropTemps(inner) => peel_drop_temps(inner),
        _ => expr,
    }
}

/// Returns the source text at the user-written call site for one span.
pub(crate) fn snippet(cx: &LateContext<'_>, span: Span) -> Option<String> {
    cx.sess()
        .source_map()
        .span_to_snippet(span.source_callsite())
        .ok()
}

/// Returns whether an expression resolves to a function with the target API path suffix.
pub(crate) fn resolved_path_ends_with(cx: &LateContext<'_>, expr: &Expr<'_>, suffix: &str) -> bool {
    // Compare the resolved definition because local functions can share the spelling.
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.qpath_res(&qpath, expr.hir_id) else {
        return false;
    };
    cx.tcx.def_path_str(def_id).ends_with(suffix)
}

/// Returns whether a path resolves to standard `IntoIterator::into_iter`.
fn resolved_into_iter(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Accept rustc's generic path rendering while retaining the trait identity check.
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.qpath_res(&qpath, expr.hir_id) else {
        return false;
    };
    // Require both the method suffix and the standard trait identity.
    let path = cx.tcx.def_path_str(def_id);
    path.ends_with("::into_iter") && path.contains("IntoIterator")
}

/// Returns the resolved definition path for a method call.
pub(crate) fn method_path<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> Option<String> {
    let ExprKind::MethodCall(..) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    Some(cx.tcx.def_path_str(def_id))
}

/// Returns whether a method call resolves to the target definition suffix.
pub(crate) fn method_path_ends_with<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    suffix: &str,
) -> bool {
    method_path(cx, expr).is_some_and(|path| path.ends_with(suffix))
}

/// Returns whether a type is the standard `Option` type.
pub(crate) fn is_option(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    cx.tcx
        .is_diagnostic_item(rustc_span::sym::Option, adt.did())
}

/// Returns whether a type is the standard `Result` type.
pub(crate) fn is_result(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    cx.tcx
        .is_diagnostic_item(rustc_span::sym::Result, adt.did())
}

/// Returns whether a type is one of the supported standard collections.
pub(crate) fn is_standard_collection(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    // Pair the item name with its defining crate to reject local lookalikes.
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    matches!(cx.tcx.crate_name(adt.did().krate).as_str(), "alloc" | "std")
        && matches!(
            cx.tcx.item_name(adt.did()).as_str(),
            "Vec" | "VecDeque" | "HashSet" | "BTreeSet" | "HashMap" | "BTreeMap"
        )
}

/// Returns whether a type is a standard sequence accepted by `extend` detection.
pub(crate) fn is_standard_sequence(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    matches!(cx.tcx.crate_name(adt.did().krate).as_str(), "alloc" | "std")
        && matches!(cx.tcx.item_name(adt.did()).as_str(), "Vec" | "VecDeque")
}

/// Returns whether a type is a slice, array, or standard `Vec`.
pub(crate) fn is_slice_like(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    // Preserve primitive slices and arrays while resolving `Vec` by its crate.
    match ty.peel_refs().kind() {
        ty::Slice(_) | ty::Array(..) => true,
        ty::Adt(adt, _) => {
            cx.tcx.item_name(adt.did()).as_str() == "Vec"
                && matches!(cx.tcx.crate_name(adt.did().krate).as_str(), "alloc" | "std")
        }
        _ => false,
    }
}

/// Emits a help-only lint diagnostic.
pub(crate) fn emit(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Keep these lints help-only until exact closure source can be rendered safely.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}
