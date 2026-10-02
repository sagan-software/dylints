#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for runtime environment reads outside configuration boundaries.
//!
//! It resolves calls that read process environment state, walks their enclosing
//! function context, and reports reads that bypass an explicit configuration
//! boundary. Generated code and recognized boundary functions remain quiet so
//! the diagnostic points at application behavior that can be moved or wrapped.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ExprKind,
    attrs::AttributeKind,
    def::{DefKind, Res},
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub RUNTIME_ENV_READ,
    Warn,
    "runtime environment read outside configuration/bootstrap code",
    RuntimeEnvRead
}

impl<'tcx> LateLintPass<'tcx> for RuntimeEnvRead {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) || allowed_function_context(cx, kind, span, local_def_id)
        {
            return;
        }

        EnvReadFinder { cx }.visit_expr(body.value);
    }
}

/// State used by the env read finder analysis.
struct EnvReadFinder<'cx, 'tcx> {
    /// cx stored for this lint's analysis.
    cx: &'cx LateContext<'tcx>,
}

impl<'tcx> Visitor<'tcx> for EnvReadFinder<'_, 'tcx> {
    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Report a resolved environment read before descending into child expressions.
        if let Some(call) = env_read_call(self.cx, expr) {
            emit_span_lint_with_help(
                self.cx,
                RUNTIME_ENV_READ,
                call.span,
                format!(
                    "`std::env::{}` reads runtime environment outside config/bootstrap code",
                    call.name
                ),
                "read environment variables in config/bootstrap, CLI entrypoints, Cargo scripts, or tests, then pass typed configuration inward",
            );
        }

        // Continue traversal so nested environment reads also receive diagnostics.
        intravisit::walk_expr(self, expr);
    }
}

/// State used by the env read call analysis.
struct EnvReadCall {
    /// name stored for this lint's analysis.
    name: &'static str,
    /// span stored for this lint's analysis.
    span: Span,
}

/// Helper for env read call analysis.
fn env_read_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<EnvReadCall> {
    let ExprKind::Call(callee, _) = expr.kind else {
        return None;
    };

    // Match the resolved std function so aliases warn and local helpers named `var` do not.
    resolved_env_read_call(cx, callee).map(|name| EnvReadCall {
        name,
        span: callee.span,
    })
}

/// Return whether resolution found env read call.
fn resolved_env_read_call(cx: &LateContext<'_>, callee: &Expr<'_>) -> Option<&'static str> {
    let ExprKind::Path(qpath) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Fn, def_id) = cx.qpath_res(&qpath, callee.hir_id) else {
        return None;
    };

    // `def_path_str` covers fully-qualified calls, imported functions, and module aliases.
    match cx.tcx.def_path_str(def_id).as_str() {
        "std::env::var" => Some("var"),
        "std::env::var_os" => Some("var_os"),
        _ => None,
    }
}

/// Helper for allowed function context analysis.
fn allowed_function_context(
    cx: &LateContext<'_>,
    kind: FnKind<'_>,
    span: Span,
    local_def_id: LocalDefId,
) -> bool {
    // Allow entrypoints and named boundaries before scanning the body for environment reads.
    cli_or_context_function(kind)
        || test_function(cx, local_def_id)
        || allowed_def_path(cx, local_def_id)
        || allowed_source_path(cx, span)
}

/// Helper for cli or context function analysis.
fn cli_or_context_function(kind: FnKind<'_>) -> bool {
    function_name(kind).is_some_and(|name| name == "main" || name_has_allowed_context(&name))
}

/// Return the function name.
fn function_name(kind: FnKind<'_>) -> Option<String> {
    match kind {
        FnKind::ItemFn(ident, ..) | FnKind::Method(ident, _) => Some(ident.name.to_ident_string()),
        FnKind::Closure => None,
    }
}

/// Helper for test function analysis.
fn test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id);
    let attrs: &[Attribute] = cx.tcx.hir_attrs(hir_id);

    // Match both ordinary tests and cfg(test) helpers directly annotated on a function.
    attrs.iter().any(|attr| {
        attr.has_name(sym::test)
            || matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_)))
            || cfg_test_attr(cx, attr)
    })
}

/// Helper for cfg test attr analysis.
fn cfg_test_attr(cx: &LateContext<'_>, attr: &Attribute) -> bool {
    if !attr.has_name(sym::cfg) && !attr.has_name(sym::cfg_attr) {
        return false;
    }

    // Attribute parsing APIs differ across nightlies; the source text is enough for this narrow
    // exemption and keeps the lint conservative if the snippet is unavailable.
    cx.sess()
        .source_map()
        .span_to_snippet(attr.span())
        .is_ok_and(|source| source.contains("test"))
}

/// Helper for allowed def path analysis.
fn allowed_def_path(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let path = cx.tcx.def_path_str(local_def_id);

    // Def paths include module names, so `config::load` and `tests::helper` are covered even when
    // the function name itself is generic.
    path.split("::")
        .any(|segment| name_has_allowed_context(segment) || test_context_name(segment))
}

/// Helper for allowed source path analysis.
fn allowed_source_path(cx: &LateContext<'_>, span: Span) -> bool {
    let Some(path) = cx
        .sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
    else {
        return false;
    };

    if path.file_name().is_some_and(|name| name == "build.rs") {
        return true;
    }

    let file_stem_allowed = path
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(name_has_allowed_context);
    let parent_allowed = path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .is_some_and(name_has_allowed_context);

    // Only inspect the file stem and immediate parent to avoid allowing every fixture in this lint
    // crate just because the checkout path contains `runtime_env_read`.
    file_stem_allowed || parent_allowed
}

/// Helper for name has allowed context analysis.
fn name_has_allowed_context(name: &str) -> bool {
    tokens(name).any(|token| {
        matches!(
            token,
            "cli"
                | "config"
                | "configuration"
                | "bootstrap"
                | "settings"
                | "setting"
                | "env"
                | "environment"
        )
    })
}

/// Return the test context name.
fn test_context_name(name: &str) -> bool {
    tokens(name).any(|token| matches!(token, "test" | "tests" | "testing"))
}

/// Helper for tokens analysis.
fn tokens(name: &str) -> impl Iterator<Item = &str> {
    name.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

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
