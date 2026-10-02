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
//! unrelated methods so suggestions remain local and machine-applicable.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, Pat, PatKind, QPath,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol, def_id::DefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub UNNECESSARY_MAP_ERR,
    Warn,
    "unnecessary `.map_err` conversion before error propagation",
    UnnecessaryMapErr
}

impl<'tcx> LateLintPass<'tcx> for UnnecessaryMapErr {
    /// Check fn for this lint.
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

/// State used by the map err visitor analysis.
struct MapErrVisitor<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
    /// function error ty stored for this lint's analysis.
    function_error_ty: Ty<'tcx>,
}

impl<'tcx> Visitor<'tcx> for MapErrVisitor<'_, 'tcx> {
    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Report a removable conversion before descending into child expressions.
        if let Some(removal_span) = unnecessary_map_err(self.cx, self.function_error_ty, expr) {
            emit_span_lint_with_help(
                self.cx,
                UNNECESSARY_MAP_ERR,
                removal_span,
                "this `.map_err` only converts the error type",
                "remove `.map_err(...)` and let `?` apply the `From` conversion",
            );
        }

        if matches!(expr.kind, ExprKind::Closure(_)) {
            return;
        }

        walk_expr(self, expr);
    }
}

/// Helper for unnecessary map err analysis.
fn unnecessary_map_err<'tcx>(
    cx: &LateContext<'tcx>,
    function_error_ty: Ty<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<Span> {
    // Prove both receiver and output are Results with the enclosing error output.
    match expr.kind {
        ExprKind::MethodCall(segment, receiver, [mapper], _call_span) => {
            Some((segment, receiver, mapper))
        }
        _ => None,
    }
    .filter(|(segment, _, _)| segment.ident.name.as_str() == "map_err")
    .and_then(|(_, receiver, mapper)| {
        result_error_ty(cx, cx.typeck_results().expr_ty(receiver))
            .map(|receiver_error_ty| (receiver, mapper, receiver_error_ty))
    })
    .and_then(|(receiver, mapper, _receiver_error_ty)| {
        result_error_ty(cx, cx.typeck_results().expr_ty(expr))
            .map(|output_error_ty| (receiver, mapper, output_error_ty))
    })
    .filter(|(_, _, output_error_ty)| *output_error_ty == function_error_ty)
    // The receiver and output being `Result` types keeps custom `map_err` methods out of scope.
    .filter(|(_, mapper, _)| conversion_mapper(cx, mapper))
    .map(|(receiver, _, _)| expr.span.with_lo(receiver.span.hi()))
}

/// Return type information for result error.
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

/// Helper for conversion mapper analysis.
fn conversion_mapper<'tcx>(cx: &LateContext<'tcx>, mapper: &'tcx Expr<'tcx>) -> bool {
    match mapper.kind {
        ExprKind::Path(qpath) => conversion_path(cx, qpath, mapper.hir_id),
        ExprKind::Closure(closure) => {
            // Only a single-call closure is equivalent to removing `map_err`.
            // Require one direct parameter binding before inspecting the closure body.
            let body = cx.tcx.hir_body(closure.body);
            let [param] = body.params else {
                return false;
            };
            let Some(param_name) = binding_name(param.pat) else {
                return false;
            };

            closure_calls_conversion(cx, body.value, param_name)
        }
        _ => false,
    }
}

/// Helper for closure calls conversion analysis.
fn closure_calls_conversion<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    param_name: Symbol,
) -> bool {
    // Accept only direct calls, `.into()`, or a transparent block tail.
    match expr.kind {
        ExprKind::Call(callee, [arg]) if path_is_binding(arg, param_name) => {
            conversion_call(cx, callee)
        }
        ExprKind::MethodCall(segment, receiver, [], _)
            if segment.ident.name.as_str() == "into" && path_is_binding(receiver, param_name) =>
        {
            true
        }
        ExprKind::Block(block, _) => block
            .expr
            .is_some_and(|tail| closure_calls_conversion(cx, tail, param_name)),
        _ => false,
    }
}

/// Helper for conversion call analysis.
fn conversion_call<'tcx>(cx: &LateContext<'tcx>, callee: &'tcx Expr<'tcx>) -> bool {
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };

    conversion_path(cx, qpath, callee.hir_id)
}

/// Helper for conversion path analysis.
fn conversion_path<'tcx>(
    cx: &LateContext<'tcx>,
    qpath: QPath<'tcx>,
    hir_id: rustc_hir::HirId,
) -> bool {
    let path_names = qpath_names(qpath);
    if path_names.as_slice() == ["Into", "into"] || path_names.as_slice() == ["From", "from"] {
        return true;
    }

    // Resolve qualified `from` paths back to the standard From trait item.
    let Some(def_id) = cx.typeck_results().qpath_res(&qpath, hir_id).opt_def_id() else {
        return false;
    };
    let path_ends_with_from = path_names.last().is_some_and(|name| name == "from");

    // A resolved `From::from` associated item proves `?` has the same conversion available.
    path_ends_with_from && from_trait_assoc(cx, def_id)
}

/// Helper for from trait assoc analysis.
fn from_trait_assoc(cx: &LateContext<'_>, def_id: DefId) -> bool {
    // Walk the associated item back to its owning trait definition.
    let Some(assoc_item) = cx.tcx.opt_associated_item(def_id) else {
        return false;
    };
    let Ok(trait_item) = assoc_item.trait_item_or_self() else {
        return false;
    };
    let Some(trait_def_id) = cx.tcx.trait_of_assoc(trait_item) else {
        return false;
    };

    // Compare trait identity so aliases and qualified syntax remain equivalent.
    cx.tcx.is_diagnostic_item(sym::From, trait_def_id)
}

/// Helper for qpath names analysis.
fn qpath_names(qpath: QPath<'_>) -> Vec<String> {
    match qpath {
        QPath::Resolved(_, path) => path
            .segments
            .iter()
            .map(|segment| segment.ident.name.to_ident_string())
            .collect(),
        QPath::TypeRelative(_, segment) => vec![segment.ident.name.to_ident_string()],
    }
}

/// Return the binding name.
const fn binding_name(pat: &Pat<'_>) -> Option<Symbol> {
    let PatKind::Binding(_mode, _hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some(ident.name)
}

/// Helper for path is binding analysis.
fn path_is_binding(expr: &Expr<'_>, name: Symbol) -> bool {
    // Require a single resolved path segment matching the closure binding.
    let ExprKind::Path(QPath::Resolved(None, path)) = expr.kind else {
        return false;
    };
    let [segment] = path.segments else {
        return false;
    };

    segment.ident.name == name
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
            let _ =
                diag.span_suggestion(span, help, String::new(), Applicability::MachineApplicable);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
