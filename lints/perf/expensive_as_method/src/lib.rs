#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores diagnostic builders after configuring them"
)]

//! A lint to check for expensive inherent `as_*` methods.
//!
//! It walks the body of each source-authored inherent `as_*` method, including
//! closures, and resolves every call to find work that a cheap borrowed view
//! never needs: owned clones, `to_string`, `to_owned`, `format!`, `str::parse`,
//! allocating `From` conversions, decode calls, and the `?` operator. Calls are
//! matched by resolved definition, so comments, strings, and cheap `Copy` or
//! reference-counted clones never count.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::ops::ControlFlow;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, LangItem, MatchSource,
    def::{DefKind, Res},
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{
    hir::nested_filter::OnlyBodies,
    ty::{self, Ty, TyCtxt},
};
use rustc_span::{
    ExpnKind, MacroKind, Span, Symbol,
    def_id::{DefId, LocalDefId},
    sym,
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub EXPENSIVE_AS_METHOD,
    Warn,
    "inherent `as_*` method does non-accessor work",
    ExpensiveAsMethod
}

impl<'tcx> LateLintPass<'tcx> for ExpensiveAsMethod {
    /// Check each inherent `as_*` method body for non-accessor work.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Restrict the naming policy to written inherent methods owned by the type.
        let FnKind::Method(ident, _sig) = kind else {
            return;
        };
        let def_id = local_def_id.to_def_id();
        let has_as_prefix = ident.name.as_str().starts_with("as_");
        let is_inherent =
            cx.tcx.impl_of_assoc(def_id).is_some() && cx.tcx.trait_impl_of_assoc(def_id).is_none();
        if !has_as_prefix || span.from_expansion() || !is_inherent {
            return;
        }

        // Point at the method name and label the first piece of work found in its body.
        let ControlFlow::Break(work) = (WorkVisitor { cx }).visit_expr(body.value) else {
            return;
        };
        cx.emit_span_lint(
            EXPENSIVE_AS_METHOD,
            ident.span,
            DiagDecorator(|diag| {
                let _ = diag.primary_message(format!(
                    "inherent method `{ident}` does work that should not hide behind `as_*`"
                ));
                let _ = diag.span_label(work.span, work.label);
                let _ = diag.help(
                    "reserve `as_*` for cheap borrowed accessors; use `to_*`, `try_*`, or a domain-specific verb for work",
                );
            }),
        );
    }
}

/// One piece of non-accessor work found in an `as_*` method body.
struct Work {
    /// Source span of the work, at the user's macro call when it comes from a macro.
    span: Span,
    /// Label that names the kind of work.
    label: &'static str,
}

/// Visitor that stops at the first non-accessor work in a body and its closures.
struct WorkVisitor<'cx, 'tcx> {
    /// Lint context used for type and resolution queries.
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for WorkVisitor<'_, 'tcx> {
    type NestedFilter = OnlyBodies;
    type Result = ControlFlow<Work>;

    /// Return the type context so closure bodies are visited.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Break at the first expression that does work.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> ControlFlow<Work> {
        // Check macro output first so `format!` is reported at the user's call site.
        if let Some(span) = format_macro_call(self.cx, expr.span) {
            return ControlFlow::Break(Work {
                span,
                label: "this `format!` allocates a `String`",
            });
        }
        if let ExprKind::Match(_, _, MatchSource::TryDesugar(_)) = expr.kind {
            return ControlFlow::Break(Work {
                span: expr.span,
                label: "this `?` makes the method fallible",
            });
        }
        if let Some(label) = call_work(self.cx, expr) {
            return ControlFlow::Break(Work {
                span: expr.span,
                label,
            });
        }

        // Continue into operands, arguments, and closure bodies.
        walk_expr(self, expr)
    }
}

/// Return the outermost source call site when a `format!` expansion produced the span.
fn format_macro_call(cx: &LateContext<'_>, span: Span) -> Option<Span> {
    span.macro_backtrace()
        .any(|expn_data| {
            matches!(expn_data.kind, ExpnKind::Macro(MacroKind::Bang, _))
                && expn_data
                    .macro_def_id
                    .is_some_and(|def_id| cx.tcx.is_diagnostic_item(sym::format_macro, def_id))
        })
        .then(|| span.source_callsite())
}

/// Return a label when the expression calls a function or method that does work.
fn call_work(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<&'static str> {
    // Resolve the callee and the value it acts on for both method and path call syntax.
    let typeck = cx.typeck_results();
    let (def_id, receiver_ty) = if let ExprKind::MethodCall(_, receiver, _, _) = expr.kind {
        (
            typeck.type_dependent_def_id(expr.hir_id)?,
            Some(typeck.expr_ty(receiver).peel_refs()),
        )
    } else if let ExprKind::Call(callee, args) = expr.kind
        && let ExprKind::Path(qpath) = callee.kind
        && let Res::Def(DefKind::Fn | DefKind::AssocFn, def_id) =
            typeck.qpath_res(&qpath, callee.hir_id)
    {
        // A path call such as `Clone::clone(&value)` acts on its first argument.
        (
            def_id,
            args.first().map(|arg| typeck.expr_ty(arg).peel_refs()),
        )
    } else {
        return None;
    };

    let trait_def_id = cx.tcx.trait_of_assoc(def_id);
    trait_def_id
        .and_then(|trait_def_id| cx.tcx.get_diagnostic_name(trait_def_id))
        .and_then(|trait_name| trait_work_label(cx, trait_name, receiver_ty, typeck.expr_ty(expr)))
        .or_else(|| named_work_label(cx, def_id))
}

/// Classify a standard trait method call as work.
///
/// The receiver and result types separate cheap calls, such as cloning a
/// `Copy` value, from calls that allocate.
fn trait_work_label<'tcx>(
    cx: &LateContext<'tcx>,
    trait_name: Symbol,
    receiver_ty: Option<Ty<'tcx>>,
    result_ty: Ty<'tcx>,
) -> Option<&'static str> {
    match trait_name.as_str() {
        // Cloning a `Copy` value or a reference-counted pointer is cheap.
        "Clone" => receiver_ty
            .is_some_and(|ty| !is_cheap_clone(cx, ty))
            .then_some("this clone copies owned data"),
        "ToOwned" => receiver_ty
            .is_some_and(|ty| !cx.tcx.type_is_copy_modulo_regions(cx.typing_env(), ty))
            .then_some("this `to_owned` allocates owned data"),
        "ToString" => Some("this `to_string` allocates a `String`"),
        "From" | "Into" => {
            is_allocating_ty(cx, result_ty).then_some("this conversion allocates owned data")
        }
        _ => None,
    }
}

/// Classify `str::parse` and functions named `decode` as work.
fn named_work_label(cx: &LateContext<'_>, def_id: DefId) -> Option<&'static str> {
    let item_name = cx.tcx.item_name(def_id);
    match item_name.as_str() {
        "parse" if is_str_inherent_method(cx, def_id) => Some("this `parse` does fallible parsing"),
        "decode" => Some("this call decodes data"),
        _ => None,
    }
}

/// Return whether cloning a value of this type is cheap.
///
/// Copying bits of a `Copy` value or bumping an `Rc` or `Arc` count is cheap.
fn is_cheap_clone<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    cx.tcx.type_is_copy_modulo_regions(cx.typing_env(), ty)
        || matches!(ty.kind(), ty::Adt(adt, _)
            if cx.tcx.is_diagnostic_item(sym::Rc, adt.did())
                || cx.tcx.is_diagnostic_item(sym::Arc, adt.did()))
}

/// Return whether the type is an owned `String` or `Vec`.
fn is_allocating_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if cx.tcx.is_lang_item(adt.did(), LangItem::String)
            || cx.tcx.is_diagnostic_item(sym::Vec, adt.did()))
}

/// Return whether the function is an inherent method of `str`.
fn is_str_inherent_method(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.impl_of_assoc(def_id).is_some_and(|impl_def_id| {
        cx.tcx
            .type_of(impl_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .is_str()
    })
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
