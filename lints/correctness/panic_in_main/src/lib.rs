#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]

//! A lint to check for panicking operations in infallible main functions.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ExprKind, FnDecl,
    attrs::AttributeKind,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{ExpnKind, MacroKind, Span, def_id::LocalDefId, sym};

/// `PANICKING_MACROS` configuration used by this lint.
const PANICKING_MACROS: &[&str] = &[
    "panic",
    "todo",
    "unimplemented",
    "assert",
    "assert_eq",
    "assert_ne",
];
/// `PANICKING_METHODS` configuration used by this lint.
const PANICKING_METHODS: &[&str] = &["unwrap", "expect"];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub PANIC_IN_MAIN,
    Warn,
    "panicking operation in an infallible `main` function",
    PanicInMain
}

impl<'tcx> LateLintPass<'tcx> for PanicInMain {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Restrict the scan to the crate entry point when it cannot propagate errors.
        if !infallible_main(cx, kind, local_def_id, span) {
            return;
        }

        // Report every panicking operation at its user-visible call site.
        for operation in PanickingOperationFinder::find(body) {
            emit_span_lint_with_help(
                cx,
                PANIC_IN_MAIN,
                operation.span,
                format!("panicking `{}` in `main`", operation.name),
                "change `fn main()` to `fn main() -> Result<(), Error>` and propagate startup errors with `?`",
            );
        }
    }
}

/// Helper for infallible main analysis.
fn infallible_main(
    cx: &LateContext<'_>,
    kind: FnKind<'_>,
    local_def_id: LocalDefId,
    span: Span,
) -> bool {
    // Require a named free function before applying entry-point checks.
    let FnKind::ItemFn(ident, ..) = kind else {
        return false;
    };
    if ident.name != sym::main
        || !crate_entry_main(cx, local_def_id)
        || test_function(cx, local_def_id)
        || build_script_main(cx, span)
    {
        return false;
    }

    !returns_result(cx, local_def_id)
}

/// Helper for crate entry main analysis.
fn crate_entry_main(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    cx.tcx
        .entry_fn(())
        .is_some_and(|(def_id, _entry_type)| def_id == local_def_id.to_def_id())
}

/// Helper for test function analysis.
fn test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id);
    let attrs: &[Attribute] = cx.tcx.hir_attrs(hir_id);

    // Match the local test-lint convention and avoid warnings in unit-test entry points.
    attrs.iter().any(|attr| {
        attr.has_name(sym::test)
            || matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_)))
    })
}

/// Helper for build script main analysis.
fn build_script_main(cx: &LateContext<'_>, span: Span) -> bool {
    cx.sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
        .is_some_and(|path| path.file_name().is_some_and(|name| name == "build.rs"))
}

/// Return whether the item returns result.
fn returns_result(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let output_ty = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder();

    result_ty(cx, output_ty)
}

/// Return type information for result.
fn result_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    let ty::Adt(adt, _) = ty.kind() else {
        return false;
    };

    cx.tcx.is_diagnostic_item(sym::Result, adt.did())
}

/// State used by the panicking operation analysis.
#[derive(Clone)]
struct PanickingOperation {
    /// name stored for this lint's analysis.
    name: &'static str,
    /// span stored for this lint's analysis.
    span: Span,
}

/// State used by the panicking operation finder analysis.
struct PanickingOperationFinder {
    /// operations stored for this lint's analysis.
    operations: Vec<PanickingOperation>,
}

impl PanickingOperationFinder {
    /// Helper for find analysis.
    fn find<'tcx>(body: &'tcx Body<'tcx>) -> Vec<PanickingOperation> {
        let mut finder = Self {
            operations: Vec::new(),
        };

        // Walk only after the caller proves this is an infallible `main`.
        finder.visit_expr(body.value);
        finder.operations
    }

    /// Helper for push unique analysis.
    fn push_unique(&mut self, operation: PanickingOperation) {
        // Macro expansion can surface several HIR nodes for one call; keep one diagnostic.
        if self.operations.iter().any(|existing| {
            existing.name == operation.name
                && existing.span.lo() == operation.span.lo()
                && existing.span.hi() == operation.span.hi()
        }) {
            return;
        }

        self.operations.push(operation);
    }
}

impl<'tcx> Visitor<'tcx> for PanickingOperationFinder {
    /// Helper for visit expr analysis.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let Some(operation) = panicking_operation(expr) {
            self.push_unique(operation);
        }

        walk_expr(self, expr);
    }
}

/// Helper for panicking operation analysis.
fn panicking_operation(expr: &Expr<'_>) -> Option<PanickingOperation> {
    if let Some(operation) = panicking_method_call(expr) {
        return Some(operation);
    }

    panicking_macro_call(expr.span)
}

/// Helper for panicking method call analysis.
fn panicking_method_call(expr: &Expr<'_>) -> Option<PanickingOperation> {
    let ExprKind::MethodCall(segment, _receiver, _args, _call_span) = expr.kind else {
        return None;
    };
    let method_name = segment.ident.name.as_str();

    PANICKING_METHODS
        .iter()
        .copied()
        .find(|candidate| *candidate == method_name)
        .map(|name| PanickingOperation {
            name,
            span: segment.ident.span,
        })
}

/// Helper for panicking macro call analysis.
fn panicking_macro_call(span: Span) -> Option<PanickingOperation> {
    let mut expn_data = span.ctxt().outer_expn_data();
    let mut found = None;

    // Walk outward so nested panic implementations prefer the user's outer macro call.
    loop {
        if let ExpnKind::Macro(MacroKind::Bang, name) = expn_data.kind
            && let Some(macro_name) = panicking_macro_name(name.as_str())
        {
            found = Some(PanickingOperation {
                name: macro_name,
                span: expn_data.call_site,
            });
        }

        if !expn_data.call_site.from_expansion() {
            return found;
        }
        // Continue through nested expansions until reaching the user's source call.
        expn_data = expn_data.call_site.ctxt().outer_expn_data();
    }
}

/// Return the panicking macro name.
fn panicking_macro_name(name: &str) -> Option<&'static str> {
    let normalized = name.strip_suffix("_macro").unwrap_or(name);
    if PANICKING_MACROS.contains(&normalized) {
        return PANICKING_MACROS
            .iter()
            .copied()
            .find(|candidate| *candidate == normalized);
    }

    match normalized {
        "core_panic" | "std_panic" => Some("panic"),
        name if name.starts_with("panic_") => Some("panic"),
        _ => None,
    }
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
