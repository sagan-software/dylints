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
use rustc_span::{
    ExpnKind, MacroKind, Span,
    def_id::{DefId, LocalDefId},
    sym,
};

/// Diagnostic items of panicking macros and the name the diagnostic reports for each.
const PANICKING_MACROS: &[(&str, &str)] = &[
    ("assert_eq_macro", "assert_eq"),
    ("assert_macro", "assert"),
    ("assert_ne_macro", "assert_ne"),
    ("core_panic_macro", "panic"),
    ("std_panic_macro", "panic"),
    ("todo_macro", "todo"),
    ("unimplemented_macro", "unimplemented"),
    ("unreachable_macro", "unreachable"),
];
/// Panicking `Option` and `Result` methods.
const PANICKING_METHODS: &[&str] = &["expect", "unwrap"];

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
        for operation in PanickingOperationFinder::find(cx, body) {
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
        || !is_crate_entry_main(cx, local_def_id)
        || test_function(cx, local_def_id)
        || build_script_main(cx, span)
    {
        return false;
    }

    !returns_result(cx, local_def_id)
}

/// Return whether this function is the compiler-selected crate entry point.
fn is_crate_entry_main(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
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

/// Visitor that collects panicking operations in source order.
struct PanickingOperationFinder<'cx, 'tcx> {
    /// Lint context used to resolve macros and methods.
    cx: &'cx LateContext<'tcx>,
    /// Panicking operations found so far, without duplicate call sites.
    operations: Vec<PanickingOperation>,
}

impl<'cx, 'tcx> PanickingOperationFinder<'cx, 'tcx> {
    /// Collect every panicking operation in the body outside nested closures.
    fn find(cx: &'cx LateContext<'tcx>, body: &'tcx Body<'tcx>) -> Vec<PanickingOperation> {
        let mut finder = Self {
            cx,
            operations: Vec::new(),
        };

        // Walk only after the caller proves this is an infallible `main`.
        finder.visit_expr(body.value);
        finder.operations
    }

    /// Record an operation unless the same call site was already recorded.
    fn push_unique(&mut self, operation: PanickingOperation) {
        // Macro expansion can surface several HIR nodes for one call; keep one diagnostic.
        let is_duplicate = self
            .operations
            .iter()
            .any(|existing| existing.name == operation.name && existing.span == operation.span);
        if !is_duplicate {
            self.operations.push(operation);
        }
    }
}

impl<'tcx> Visitor<'tcx> for PanickingOperationFinder<'_, 'tcx> {
    /// Record a panicking operation, then continue into child expressions.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if let Some(operation) = panicking_operation(self.cx, expr) {
            self.push_unique(operation);
        }

        walk_expr(self, expr);
    }
}

/// Return the panicking method call or macro call that produced the expression.
fn panicking_operation(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<PanickingOperation> {
    if let Some(operation) = panicking_method_call(cx, expr) {
        return Some(operation);
    }

    // The backtrace runs from the innermost expansion outward, so the last match is the
    // user's macro call rather than a nested panic implementation.
    expr.span
        .macro_backtrace()
        .filter(|expn_data| matches!(expn_data.kind, ExpnKind::Macro(MacroKind::Bang, _)))
        .filter_map(|expn_data| {
            let name = panicking_macro_name(cx, expn_data.macro_def_id?)?;
            Some(PanickingOperation {
                name,
                span: expn_data.call_site,
            })
        })
        .last()
}

/// Return an `Option` or `Result` `unwrap` or `expect` call.
fn panicking_method_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<PanickingOperation> {
    // Resolve the called method before comparing its name.
    let ExprKind::MethodCall(segment, _receiver, _args, _call_span) = expr.kind else {
        return None;
    };
    let method_def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let method_name = cx.tcx.item_name(method_def_id);
    let name = PANICKING_METHODS
        .iter()
        .copied()
        .find(|name| *name == method_name.as_str())?;

    // Resolve the method to an inherent impl on `Option` or `Result`, whatever its spelling.
    let impl_def_id = cx.tcx.impl_of_assoc(method_def_id)?;
    let self_ty = cx
        .tcx
        .type_of(impl_def_id)
        .instantiate_identity()
        .skip_norm_wip();
    matches!(self_ty.kind(), ty::Adt(adt, _)
        if cx.tcx.is_diagnostic_item(sym::Option, adt.did())
            || cx.tcx.is_diagnostic_item(sym::Result, adt.did()))
    .then_some(PanickingOperation {
        name,
        span: segment.ident.span,
    })
}

/// Return the reported name of a standard panicking macro, matched by diagnostic item.
fn panicking_macro_name(cx: &LateContext<'_>, def_id: DefId) -> Option<&'static str> {
    let diagnostic_name = cx.tcx.get_diagnostic_name(def_id)?;

    PANICKING_MACROS
        .iter()
        .find(|(item, _)| *item == diagnostic_name.as_str())
        .map(|(_, name)| *name)
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
