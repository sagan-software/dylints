#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for logging inside standard conversion impls.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, ImplItem, ImplItemImplKind, ImplItemKind, QPath,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{ExpnKind, MacroKind, Span, def_id::DefId, sym};

/// `LOG_MACROS` configuration used by this lint.
const LOG_MACROS: &[&str] = &["debug", "eprintln", "error", "info", "trace", "warn"];
/// `LOG_FUNCTION_NAMES` configuration used by this lint.
const LOG_FUNCTION_NAMES: &[&str] = &[
    "debug",
    "error",
    "info",
    "log_error",
    "log_warn",
    "trace",
    "warn",
];
/// `LOG_MODULE_NAMES` configuration used by this lint.
const LOG_MODULE_NAMES: &[&str] = &["kslog", "log", "tracing"];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub LOGGED_CONVERSION_IMPL,
    Warn,
    "standard conversion impl logs inside the conversion body",
    LoggedConversionImpl
}

impl<'tcx> LateLintPass<'tcx> for LoggedConversionImpl {
    /// Check impl item for this lint.
    fn check_impl_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx ImplItem<'tcx>) {
        let Some(body) = conversion_impl_body(cx, item) else {
            return;
        };

        if let Some(span) = first_logging_call(cx, item, body) {
            emit_span_lint_with_help(
                cx,
                LOGGED_CONVERSION_IMPL,
                span,
                "this standard conversion impl logs while converting",
                "return a value or error and let the caller choose where to log",
            );
        }
    }
}

/// Helper for conversion impl body analysis.
fn conversion_impl_body<'tcx>(
    cx: &LateContext<'tcx>,
    item: &'tcx ImplItem<'tcx>,
) -> Option<&'tcx Body<'tcx>> {
    // Require a function item before checking its enclosing trait implementation.
    let ImplItemKind::Fn(_sig, body_id) = item.kind else {
        return None;
    };
    if !matches!(item.impl_kind, ImplItemImplKind::Trait { .. }) || !conversion_impl_trait(cx, item)
    {
        return None;
    }

    Some(cx.tcx.hir_body(body_id))
}

/// Helper for conversion impl trait analysis.
fn conversion_impl_trait(cx: &LateContext<'_>, item: &ImplItem<'_>) -> bool {
    // The parent of an impl item is the impl block. Querying the impl trait ref catches every
    // method in the impl, including `From::from`, without relying on associated-item metadata.
    let impl_def_id = cx.tcx.parent(item.owner_id.def_id.to_def_id());
    let trait_def_id = cx.tcx.impl_trait_ref(impl_def_id).skip_binder().def_id;

    conversion_trait(cx, trait_def_id)
}

/// Helper for conversion trait analysis.
fn conversion_trait(cx: &LateContext<'_>, trait_def_id: DefId) -> bool {
    let trait_path = cx.tcx.def_path_str(trait_def_id);

    cx.tcx.is_diagnostic_item(sym::From, trait_def_id)
        || cx.tcx.is_diagnostic_item(sym::TryFrom, trait_def_id)
        || matches!(
            trait_path.as_str(),
            "core::convert::From"
                | "core::convert::TryFrom"
                | "core::str::traits::FromStr"
                | "std::convert::From"
                | "std::convert::TryFrom"
                | "std::str::FromStr"
        )
}

/// Return the first logging call.
fn first_logging_call(cx: &LateContext<'_>, item: &ImplItem<'_>, body: &Body<'_>) -> Option<Span> {
    let mut visitor = LoggingVisitor { span: None };
    visitor.visit_expr(body.value);
    visitor
        .span
        .or_else(|| source_logging_call(cx, item.span).then_some(item.span))
}

/// Helper for source logging call analysis.
fn source_logging_call(cx: &LateContext<'_>, span: Span) -> bool {
    let Ok(source) = cx.sess().source_map().span_to_snippet(span) else {
        return false;
    };

    // Macro expansions do not always leave a stable HIR callee; scan only the already-resolved
    // conversion impl item source for explicit logging macro syntax.
    LOG_MACROS
        .iter()
        .any(|name| source.contains(&format!("{name}!")) || source.contains(&format!("{name} !")))
        || LOG_MODULE_NAMES.iter().any(|module| {
            LOG_MACROS.iter().any(|name| {
                source.contains(&format!("{module}::{name}!"))
                    || source.contains(&format!("{module}::{name} !"))
            })
        })
}

/// State used by the logging visitor analysis.
struct LoggingVisitor {
    /// span stored for this lint's analysis.
    span: Option<Span>,
}

impl<'tcx> Visitor<'tcx> for LoggingVisitor {
    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Stop traversal after locating the first user-visible logging call.
        if self.span.is_some() {
            return;
        }
        // Record macro or function logging before walking child expressions.
        if let Some(span) = logging_call(expr) {
            self.span = Some(span);
            return;
        }

        walk_expr(self, expr);
    }

    /// Helper for visit nested body analysis.
    fn visit_nested_body(&mut self, _body: rustc_hir::BodyId) {
        // Nested closures and async blocks can have their own side-effect rules; keep this lint
        // focused on logging directly in the conversion method's control flow.
    }
}

/// Helper for logging call analysis.
fn logging_call(expr: &Expr<'_>) -> Option<Span> {
    logging_macro_call(expr.span).or_else(|| logging_function_call(expr).then_some(expr.span))
}

/// Helper for logging macro call analysis.
fn logging_macro_call(span: Span) -> Option<Span> {
    let mut expn_data = span.ctxt().outer_expn_data();

    // Macro-expanded logging calls can lower to several HIR nodes; use the outer call site name.
    loop {
        if let ExpnKind::Macro(MacroKind::Bang, name) = expn_data.kind {
            let macro_name = normalize_path_name(name.as_str());
            if LOG_MACROS.contains(&macro_name) {
                return Some(expn_data.call_site);
            }
        }

        if !expn_data.call_site.from_expansion() {
            return None;
        }
        // Continue outward until reaching a source-authored macro call.
        expn_data = expn_data.call_site.ctxt().outer_expn_data();
    }
}

/// Helper for logging function call analysis.
fn logging_function_call(expr: &Expr<'_>) -> bool {
    let ExprKind::Call(callee, _args) = expr.kind else {
        return false;
    };
    let ExprKind::Path(qpath) = callee.kind else {
        return false;
    };
    let names = qpath_names(qpath);
    let Some(last_name) = names.last().map(String::as_str) else {
        return false;
    };

    // Require both a known logging function and a recognized logging namespace.
    // Keep function-call matching namespaced so ordinary `warn(...)` helpers are not flagged.
    LOG_FUNCTION_NAMES.contains(&last_name)
        && names
            .iter()
            .any(|name| LOG_MODULE_NAMES.contains(&name.as_str()))
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

/// Return the normalized path name.
fn normalize_path_name(name: &str) -> &str {
    name.rsplit("::")
        .next()
        .unwrap_or(name)
        .strip_suffix("_macro")
        .unwrap_or_else(|| name.rsplit("::").next().unwrap_or(name))
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to match the rest of this lint suite.
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
