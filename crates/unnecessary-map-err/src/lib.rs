#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]

//! A lint to check for unnecessary `map_err` conversions before error propagation.
//!
//! It compares a standard `Result` receiver and output with the enclosing
//! function's error type, then recognizes a mapper that only applies the
//! existing `From` conversion used by `?`. The visitor skips closures and
//! unrelated methods. It offers a machine-applicable removal only when a `?`
//! operator consumes the call, so the rewrite keeps the same propagation.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, HirId, MatchSource, Node, PatKind, QPath,
    def::Res,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_infer::infer::TyCtxtInferExt as _;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, def_id::DefId, sym};
use rustc_trait_selection::infer::InferCtxtExt as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub UNNECESSARY_MAP_ERR,
    Warn,
    "unnecessary `.map_err` conversion before error propagation",
    UnnecessaryMapErr
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryMapErr {
    /// Walks a function body whose return type is a standard `Result`.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        _local_def_id: rustc_span::def_id::LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }

        let Some(function_error_ty) = result_error_ty(cx, cx.typeck_results().expr_ty(body.value))
        else {
            return;
        };

        // Carry the function's error type while walking so nested calls can be compared to it.
        MapErrVisitor {
            cx,
            function_error_ty,
        }
        .visit_expr(body.value);
    }
}

/// Visitor that reports conversion-only `map_err` calls in one function body.
struct MapErrVisitor<'cx, 'tcx> {
    /// Lint context of the visited function.
    cx: &'cx LateContext<'tcx>,
    /// Error type of the enclosing function's `Result`.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for MapErrVisitor<'_, 'tcx> {
    /// Reports a matching call, then descends into everything except closures.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Report a removable conversion before descending into child expressions.
        if let Some(receiver) = unnecessary_map_err(self.cx, self.function_error_ty, expr) {
            emit_map_err_lint(self.cx, expr, receiver);
        }

        // A `?` inside a closure propagates to the closure, not to this function.
        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// Returns the receiver of a conversion-only `Result::map_err` call.
fn unnecessary_map_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Prove both receiver and output are Results with the enclosing error output.
    let ExprKind::MethodCall(_, receiver, [mapper], _) = expr.kind else {
        return None;
    };
    let receiver_error_ty = result_error_ty(cx, cx.typeck_results().expr_ty(receiver))?;
    let output_error_ty = result_error_ty(cx, cx.typeck_results().expr_ty(expr))?;

    // Only `Result` has inherent methods on `Result`, so the name identifies `Result::map_err`.
    let is_result_map_err = cx
        .typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|def_id| {
            cx.tcx.trait_of_assoc(def_id).is_none()
                && cx.tcx.item_name(def_id) == Symbol::intern("map_err")
        });

    // `?` converts through `From`, so an `Into`-only conversion is not equivalent.
    (is_result_map_err
        && output_error_ty == function_error_ty
        && conversion_mapper(cx, mapper)
        && is_from_conversion(cx, function_error_ty, receiver_error_ty))
    .then_some(receiver)
}

/// Return whether `target: From<source>` holds in the current function's environment.
fn is_from_conversion<'tcx>(cx: &LateContext<'tcx>, target: Ty<'tcx>, source: Ty<'tcx>) -> bool {
    // Ask the trait solver because `?` resolves the same obligation.
    cx.tcx
        .get_diagnostic_item(sym::From)
        .is_some_and(|from_trait| {
            cx.tcx
                .infer_ctxt()
                .build(cx.typing_mode())
                .type_implements_trait(from_trait, [target, source], cx.param_env)
                .must_apply_modulo_regions()
        })
}

/// Return whether `expr` is the operand of a `?` operator.
///
/// HIR lowers `operand?` to `match Try::branch(operand) { .. }` with a
/// `TryDesugar` source, so the operand's parent is the `branch` call and its
/// grandparent is that match.
fn is_try_operand(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let mut parents = cx.tcx.hir_parent_iter(expr.hir_id);
    // Require the desugared `Try::branch(operand)` call directly above the operand.
    let Some((_, Node::Expr(branch_call))) = parents.next() else {
        return false;
    };
    if !matches!(branch_call.kind, ExprKind::Call(_, [argument]) if argument.hir_id == expr.hir_id)
    {
        return false;
    }

    // Require the `?` match that scrutinizes that call.
    matches!(
        parents.next(),
        Some((_, Node::Expr(Expr {
            kind: ExprKind::Match(scrutinee, _, MatchSource::TryDesugar(_)),
            ..
        }))) if scrutinee.hir_id == branch_call.hir_id
    )
}

/// Returns the error type argument of a standard `Result`.
fn result_error_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    // Extract the error argument only from the standard Result diagnostic item.
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    if !cx.tcx.is_diagnostic_item(sym::Result, adt.did()) {
        return None;
    }

    Some(args.type_at(1))
}

/// Returns whether a mapper only applies `From::from` or `Into::into`.
fn conversion_mapper<'tcx>(cx: &LateContext<'tcx>, mapper: &'tcx Expr<'tcx>) -> bool {
    match mapper.kind {
        ExprKind::Path(ref qpath) => conversion_path(cx, qpath, mapper.hir_id),
        ExprKind::Closure(closure) => {
            // Only a single-call closure is equivalent to removing `map_err`.
            // A `map_err` closure has one parameter; require it to be a plain binding.
            let body = cx.tcx.hir_body(closure.body);
            body.params
                .first()
                .and_then(|param| match param.pat.kind {
                    PatKind::Binding(_, param_id, _, None) => Some(param_id),
                    _ => None,
                })
                .is_some_and(|param_id| closure_calls_conversion(cx, body.value, param_id))
        }
        _ => false,
    }
}

/// Returns whether a closure body only converts its parameter.
fn closure_calls_conversion<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    param_id: HirId,
) -> bool {
    // Closure bodies share the enclosing function's typeck results.
    let typeck = cx.typeck_results();

    // Accept only direct calls, `.into()`, or a statement-free block tail.
    match expr.kind {
        ExprKind::Call(callee, [arg]) if is_local(cx, arg, param_id) => {
            matches!(callee.kind, ExprKind::Path(ref qpath) if conversion_path(cx, qpath, callee.hir_id))
        }
        ExprKind::MethodCall(_, receiver, [], _) if is_local(cx, receiver, param_id) => typeck
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|def_id| conversion_trait_item(cx, def_id)),
        ExprKind::Block(block, _) if block.stmts.is_empty() => block
            .expr
            .is_some_and(|tail| closure_calls_conversion(cx, tail, param_id)),
        _ => false,
    }
}

/// Returns whether a path resolves to `From::from` or `Into::into`.
fn conversion_path<'tcx>(cx: &LateContext<'tcx>, qpath: &QPath<'tcx>, hir_id: HirId) -> bool {
    cx.qpath_res(qpath, hir_id)
        .opt_def_id()
        .is_some_and(|def_id| conversion_trait_item(cx, def_id))
}

/// Returns whether an associated item implements `From::from` or `Into::into`.
fn conversion_trait_item(cx: &LateContext<'_>, def_id: DefId) -> bool {
    // Walk an impl item back to its trait item, then to the owning trait.
    let Some(trait_item) = cx
        .tcx
        .opt_associated_item(def_id)
        .and_then(|item| item.trait_item_or_self().ok())
    else {
        return false;
    };

    // Compare trait identity so aliases and qualified syntax remain equivalent.
    cx.tcx.trait_of_assoc(trait_item).is_some_and(|trait_id| {
        cx.tcx.is_diagnostic_item(sym::From, trait_id)
            || cx.tcx.is_diagnostic_item(sym::Into, trait_id)
    })
}

/// Returns whether an expression is a path to one local binding.
fn is_local(cx: &LateContext<'_>, expr: &Expr<'_>, local_id: HirId) -> bool {
    matches!(
        expr.kind,
        ExprKind::Path(ref qpath)
            if matches!(cx.qpath_res(qpath, expr.hir_id), Res::Local(id) if id == local_id)
    )
}

/// Emit the lint, with a machine-applicable removal only before `?`.
///
/// Without a following `?`, deleting `.map_err(..)` would return the receiver's
/// error type unchanged, so the diagnostic only explains the rewrite.
fn emit_map_err_lint(cx: &LateContext<'_>, expr: &Expr<'_>, receiver: &Expr<'_>) {
    // A receiver from another expansion has no user-written boundary to cut at.
    let shares_context = receiver.span.eq_ctxt(expr.span);
    let span = if shares_context {
        expr.span.with_lo(receiver.span.hi())
    } else {
        expr.span
    };

    // Macro bodies are shared by every expansion, so only user-written calls get a fix.
    let is_fixable = shares_context && !expr.span.from_expansion() && is_try_operand(cx, expr);
    cx.emit_span_lint(
        UNNECESSARY_MAP_ERR,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message("this `.map_err` only converts the error type");
            if is_fixable {
                let _ = diag.span_suggestion(
                    span,
                    "remove `.map_err(...)` and let `?` apply the `From` conversion",
                    String::new(),
                    Applicability::MachineApplicable,
                );
            } else {
                let _ = diag.help(
                    "remove `.map_err(...)` and propagate the error with `?`, as in `Ok(value?)`",
                );
            }
        }),
    );
}

/// Runs the UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
