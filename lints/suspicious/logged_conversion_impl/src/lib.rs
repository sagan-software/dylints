#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for logging inside standard conversion impls.
//!
//! It resolves the implemented trait of each method to `From`, `TryFrom`, or
//! `FromStr`, then walks the method body and its closures for calls to
//! `eprintln!`, `log` or `tracing` event macros, and functions that resolve to a
//! `kslog`, `log`, or `tracing` path. Every check uses resolved definitions, so
//! local lookalike macros and text in comments or strings never match.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

/// The UI examples depend on `tracing` to exercise real logging macro expansions.
#[cfg(test)]
use tracing as _;

use std::ops::ControlFlow;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl,
    def::{DefKind, Res},
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{hir::nested_filter::OnlyBodies, ty::TyCtxt};
use rustc_span::{
    ExpnKind, MacroKind, Span,
    def_id::{DefId, LocalDefId},
    sym,
};

/// Crates and module names whose logging macros and functions the lint recognizes.
const LOG_NAMESPACES: &[&str] = &["kslog", "log", "tracing"];
/// Event macro names exported by the `log` and `tracing` crates.
const LOG_MACROS: &[&str] = &["debug", "error", "event", "info", "log", "trace", "warn"];
/// Logging function names recognized inside a logging namespace.
const LOG_FUNCTION_NAMES: &[&str] = &[
    "debug",
    "error",
    "info",
    "log_error",
    "log_warn",
    "trace",
    "warn",
];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LOGGED_CONVERSION_IMPL,
    Warn,
    "standard conversion impl logs inside the conversion body",
    LoggedConversionImpl
}

impl<'tcx> LateLintPass<'tcx> for LoggedConversionImpl {
    /// Check each method of a standard conversion impl for a logging call.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Generated impls cannot be edited at the reported site, so only check written methods.
        if !matches!(kind, FnKind::Method(..))
            || span.from_expansion()
            || !conversion_impl_method(cx, local_def_id)
        {
            return;
        }

        // Report only the first logging call so one method produces one diagnostic.
        let mut visitor = LoggingVisitor { cx };
        if let ControlFlow::Break(log_span) = visitor.visit_expr(body.value) {
            cx.emit_span_lint(
                LOGGED_CONVERSION_IMPL,
                log_span,
                DiagDecorator(|diag| {
                    let _ =
                        diag.primary_message("this standard conversion impl logs while converting");
                    let _ =
                        diag.help("return a value or error and let the caller choose where to log");
                }),
            );
        }
    }
}

/// Return whether the method belongs to an impl of `From`, `TryFrom`, or `FromStr`.
fn conversion_impl_method(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let Some(impl_def_id) = cx.tcx.trait_impl_of_assoc(local_def_id.to_def_id()) else {
        return false;
    };
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    // `FromStr` has no diagnostic item, so match its defining crate and item name instead.
    matches!(
        cx.tcx.get_diagnostic_name(trait_def_id),
        Some(sym::From | sym::TryFrom)
    ) || (cx.tcx.crate_name(trait_def_id.krate) == sym::core
        && cx.tcx.item_name(trait_def_id).as_str() == "FromStr")
}

/// Visitor that stops at the first logging call in a body, including closure bodies.
struct LoggingVisitor<'cx, 'tcx> {
    /// Lint context used to resolve macros and callees.
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for LoggingVisitor<'_, 'tcx> {
    type NestedFilter = OnlyBodies;
    type Result = ControlFlow<Span>;

    /// Return the type context so closures and async blocks are visited.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Break with the user-visible span of the first logging call.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> ControlFlow<Span> {
        // Report a macro at its source call site and a function call at the call itself.
        if let Some(span) = logging_macro_call(self.cx, expr.span) {
            return ControlFlow::Break(span);
        }
        if logging_function_call(self.cx, expr) {
            return ControlFlow::Break(expr.span);
        }

        walk_expr(self, expr)
    }
}

/// Return the outermost source call site when a logging macro produced the span.
fn logging_macro_call(cx: &LateContext<'_>, span: Span) -> Option<Span> {
    span.macro_backtrace()
        .any(|expn_data| {
            matches!(expn_data.kind, ExpnKind::Macro(MacroKind::Bang, _))
                && expn_data
                    .macro_def_id
                    .is_some_and(|def_id| logging_macro(cx, def_id))
        })
        .then(|| span.source_callsite())
}

/// Return whether a macro definition is `eprintln!` or a `log` or `tracing` event macro.
fn logging_macro(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx
        .get_diagnostic_name(def_id)
        .is_some_and(|name| name.as_str() == "eprintln_macro")
        || (LOG_NAMESPACES.contains(&cx.tcx.crate_name(def_id.krate).as_str())
            && LOG_MACROS.contains(&cx.tcx.item_name(def_id).as_str()))
}

/// Return whether the expression calls a function that resolves to a logging namespace.
fn logging_function_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Only a call through a path can resolve to a named logging function.
    let ExprKind::Call(callee, _args) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.typeck_results().qpath_res(&qpath, callee.hir_id) else {
        return false;
    };

    // Resolve through imports and renames, then require a logging crate or module in the path.
    LOG_FUNCTION_NAMES.contains(&cx.tcx.item_name(def_id).as_str())
        && (LOG_NAMESPACES.contains(&cx.tcx.crate_name(def_id.krate).as_str())
            || cx.tcx.def_path(def_id).data.iter().any(|segment| {
                segment
                    .data
                    .get_opt_name()
                    .is_some_and(|name| LOG_NAMESPACES.contains(&name.as_str()))
            }))
}

/// Run the UI examples, which depend on the real `tracing` crate.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
