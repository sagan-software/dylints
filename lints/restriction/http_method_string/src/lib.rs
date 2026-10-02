#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only string HTTP method fields are relevant and diagnostics are configured in place"
)]

//! A lint to check for HTTP methods stored as strings.
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
    Body, FieldDef, FnDecl, FnRetTy, Param, Pat, PatKind, TraitFn, TraitItem, TraitItemKind, Ty,
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{
    Ident, Span,
    def_id::{DefId, LocalDefId},
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub HTTP_METHOD_STRING,
    Warn,
    "HTTP method stored as a string",
    HttpMethodString
}

impl<'tcx> LateLintPass<'tcx> for HttpMethodString {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        let field_name = field.ident.name.to_ident_string();

        if http_method_name(&field_name)
            && let Some(string_ty) = string_ty(
                cx,
                cx.tcx
                    .type_of(field.def_id)
                    .instantiate_identity()
                    .skip_norm_wip(),
            )
        {
            emit_http_method_lint(cx, field.ty.span, "field", &field_name, string_ty);
        }
    }

    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }

        // Body parameters carry source names, while the declaration carries the source types.
        check_params(cx, decl.inputs, body.params);

        if let Some(name) = fn_name(kind) {
            check_return(cx, decl, local_def_id, &name);
        }
    }

    /// Check trait item for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let TraitItemKind::Fn(sig, TraitFn::Required(arg_names)) = item.kind else {
            return;
        };

        // Required trait methods have no body, so their parameter identifiers live on `TraitFn`.
        check_trait_required_params(cx, sig.decl.inputs, arg_names, item.owner_id.def_id);
        check_trait_return(
            cx,
            sig.decl,
            item.owner_id.def_id,
            &item.ident.name.to_ident_string(),
        );
    }
}

/// Check params for this lint.
fn check_params(cx: &LateContext<'_>, inputs: &[Ty<'_>], params: &[Param<'_>]) {
    // Pair each written parameter type with its resolved binding and rustc type.
    for (ty, param) in inputs.iter().zip(params) {
        let Some(name) = binding_name(param.pat) else {
            continue;
        };

        check_named_ty(
            cx,
            "parameter",
            &name,
            ty.span,
            cx.typeck_results().node_type(param.hir_id),
        );
    }
}

/// Check trait required params for this lint.
fn check_trait_required_params(
    cx: &LateContext<'_>,
    inputs: &[Ty<'_>],
    arg_names: &[Option<Ident>],
    local_def_id: LocalDefId,
) {
    // Resolve the trait method signature because required methods have no body type table.
    let sig = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_binder();

    // Pair written trait parameters with their resolved signature types.
    for ((ty, maybe_ident), rustc_ty) in inputs.iter().zip(arg_names).zip(sig.inputs()) {
        let Some(ident) = maybe_ident else {
            continue;
        };

        check_named_ty(
            cx,
            "parameter",
            &ident.name.to_ident_string(),
            ty.span,
            *rustc_ty,
        );
    }
}

/// Check return for this lint.
fn check_return(cx: &LateContext<'_>, decl: &FnDecl<'_>, local_def_id: LocalDefId, name: &str) {
    let FnRetTy::Return(output) = decl.output else {
        return;
    };

    let output_ty = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder();

    check_named_ty(cx, "return value", name, output.span, output_ty);
}

/// Check trait return for this lint.
fn check_trait_return(
    cx: &LateContext<'_>,
    decl: &FnDecl<'_>,
    local_def_id: LocalDefId,
    name: &str,
) {
    let FnRetTy::Return(output) = decl.output else {
        return;
    };

    let output_ty = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder();

    check_named_ty(cx, "return value", name, output.span, output_ty);
}

/// Check named ty for this lint.
fn check_named_ty(
    cx: &LateContext<'_>,
    label: &'static str,
    name: &str,
    span: Span,
    ty: ty::Ty<'_>,
) {
    if http_method_name(name)
        && let Some(string_ty) = string_ty(cx, ty)
    {
        emit_http_method_lint(cx, span, label, name, string_ty);
    }
}

/// Emit the http method lint diagnostic.
fn emit_http_method_lint(
    cx: &LateContext<'_>,
    span: Span,
    label: &'static str,
    name: &str,
    string_ty: &'static str,
) {
    // Name the affected API position and binding in the primary diagnostic.
    // Recommend a typed HTTP method at the boundary where the string appears.
    emit_span_lint_with_help(
        cx,
        HTTP_METHOD_STRING,
        span,
        format!("HTTP method {label} `{name}` uses `{string_ty}`"),
        "use `http::Method` or `reqwest::Method`",
    );
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

/// Return the fn name.
fn fn_name(kind: FnKind<'_>) -> Option<String> {
    match kind {
        FnKind::ItemFn(ident, ..) | FnKind::Method(ident, _) => Some(ident.name.to_ident_string()),
        FnKind::Closure => None,
    }
}

/// Return the binding name.
fn binding_name(pat: &Pat<'_>) -> Option<String> {
    // Only simple bindings have a single API name; destructured patterns stay out of scope.
    let PatKind::Binding(_mode, _hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some(ident.name.to_ident_string())
}

/// Return the http method name.
fn http_method_name(name: &str) -> bool {
    // Match exact API vocabulary so names such as `payment_method` and `method_name` do not warn.
    matches!(name, "method" | "http_method" | "request_method" | "verb")
}

/// Return type information for string.
fn string_ty(cx: &LateContext<'_>, ty: ty::Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, _) if string_def_id(cx, adt.did()) => Some("String"),
        ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Str) => Some("&str"),
        _ => None,
    }
}

/// Helper for string def id analysis.
fn string_def_id(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.def_path_str(def_id).as_str(),
        "alloc::string::String" | "std::string::String"
    )
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
