#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only secret-bearing raw types are relevant and diagnostics are configured in place"
)]

//! A lint to check for secret-bearing names stored in raw string or byte types.
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
    Body, FieldDef, FnDecl, LetStmt, LocalSource, Param, Pat, PatKind, TraitFn, TraitItem,
    TraitItemKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Ident, Span, def_id::LocalDefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SECRET_RAW_TYPE,
    Warn,
    "secret-bearing name uses a raw string or byte type",
    SecretRawType
}

impl<'tcx> LateLintPass<'tcx> for SecretRawType {
    /// Check field def for this lint.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        // Restrict name checks to named fields with independent identifiers.
        if field.is_positional() {
            return;
        }

        // Resolve the field type only when its name denotes secret material.
        let field_name = field.ident.name.to_ident_string();
        if secret_name(&field_name) {
            let ty = cx
                .tcx
                .type_of(field.def_id)
                .instantiate_identity()
                .skip_norm_wip();
            check_named_ty(cx, "field", field.ident.span, &field_name, ty);
        }
    }

    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        _local_def_id: LocalDefId,
    ) {
        if matches!(kind, FnKind::Closure) {
            return;
        }

        check_body_params(cx, body.params);
    }

    /// Check local for this lint.
    fn check_local(&mut self, cx: &LateContext<'tcx>, local: &'tcx LetStmt<'tcx>) {
        // Ignore compiler-generated locals and destructuring patterns.
        if !matches!(local.source, LocalSource::Normal) {
            return;
        }

        let Some(ident) = binding_ident(local.pat) else {
            return;
        };

        // Check the inferred type only when the local name denotes secret material.
        let name = ident.name.to_ident_string();
        if secret_name(&name) {
            check_named_ty(
                cx,
                "local variable",
                ident.span,
                &name,
                cx.typeck_results().node_type(local.hir_id),
            );
        }
    }

    /// Check trait item for this lint.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let TraitItemKind::Fn(sig, TraitFn::Required(arg_names)) = item.kind else {
            return;
        };

        check_trait_required_params(cx, sig.decl.inputs, arg_names);
    }
}

/// Check body params for this lint.
fn check_body_params(cx: &LateContext<'_>, params: &[Param<'_>]) {
    // Inspect direct parameter bindings and ignore destructuring patterns.
    for param in params {
        let Some(ident) = binding_ident(param.pat) else {
            continue;
        };

        // Check the resolved parameter type only for secret-bearing names.
        let name = ident.name.to_ident_string();
        if secret_name(&name) {
            check_named_ty(
                cx,
                "parameter",
                ident.span,
                &name,
                cx.typeck_results().node_type(param.hir_id),
            );
        }
    }
}

/// Check trait required params for this lint.
fn check_trait_required_params(
    cx: &LateContext<'_>,
    inputs: &[rustc_hir::Ty<'_>],
    arg_names: &[Option<Ident>],
) {
    // Align required-trait parameter types with their optional source names.
    for (ty, maybe_ident) in inputs.iter().zip(arg_names) {
        let Some(ident) = maybe_ident else {
            continue;
        };

        // Emit only when both the name and HIR type prove a raw secret boundary.
        let name = ident.name.to_ident_string();
        if secret_name(&name)
            && let Some(raw_ty) = raw_hir_secret_ty(ty)
        {
            emit_secret_lint(cx, ident.span, "parameter", &name, raw_ty);
        }
    }
}

/// Check named ty for this lint.
fn check_named_ty(cx: &LateContext<'_>, label: &'static str, span: Span, name: &str, ty: Ty<'_>) {
    if let Some(raw_ty) = raw_secret_ty(cx, ty) {
        emit_secret_lint(cx, span, label, name, raw_ty);
    }
}

/// Emit the secret lint diagnostic.
fn emit_secret_lint(
    cx: &LateContext<'_>,
    span: Span,
    label: &'static str,
    name: &str,
    raw_ty: &'static str,
) {
    // Build a precise message that names the boundary and its raw type.
    // Recommend a wrapper that prevents accidental disclosure at later boundaries.
    emit_span_lint_with_help(
        cx,
        SECRET_RAW_TYPE,
        span,
        format!("secret-bearing {label} `{name}` uses raw `{raw_ty}`"),
        "use `secrecy::SecretString`, `SecretBox`, or a project-specific secret newtype",
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

    // Use rustc's native diagnostic decorator to match the rest of the lint suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for binding ident analysis.
const fn binding_ident(pat: &Pat<'_>) -> Option<Ident> {
    // Destructured patterns introduce several names; keep diagnostics to one clear binding.
    let PatKind::Binding(_mode, _hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some(ident)
}

/// Return the secret name.
fn secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let tokens = lower
        .split('_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    // Match explicit secret vocabulary, including the common two-token `api_key` form.
    tokens
        .iter()
        .any(|token| matches!(*token, "token" | "password" | "secret"))
        || tokens.windows(2).any(|window| window == ["api", "key"])
        || matches!(lower.as_str(), "apikey")
}

/// Return type information for raw secret.
fn raw_secret_ty(cx: &LateContext<'_>, ty: Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, args) => raw_adt_secret_ty(cx, *adt, args),
        ty::Ref(_, inner, _) => raw_ref_secret_ty(*inner),
        ty::Array(element, _) if u8_ty(*element) => Some("[u8; N]"),
        ty::Slice(element) if u8_ty(*element) => Some("[u8]"),
        _ => None,
    }
}

/// Return the supported raw representation for one resolved standard ADT.
fn raw_adt_secret_ty(
    cx: &LateContext<'_>,
    adt: ty::AdtDef<'_>,
    args: &ty::List<ty::GenericArg<'_>>,
) -> Option<&'static str> {
    // Match the resolved ADT and its first type argument before naming a raw secret shape.
    let item_name = cx.tcx.item_name(adt.did());
    let name = item_name.as_str();
    let is_vec_u8 = name == "Vec"
        && args
            .iter()
            .next()
            .and_then(ty::GenericArg::as_type)
            .is_some_and(u8_ty);
    let is_box_u8_slice = name == "Box"
        && args
            .iter()
            .next()
            .and_then(ty::GenericArg::as_type)
            .is_some_and(slice_u8_ty);
    (name == "String")
        .then_some("String")
        .or_else(|| is_vec_u8.then_some("Vec<u8>"))
        .or_else(|| is_box_u8_slice.then_some("Box<[u8]>"))
}

/// Return type information for raw ref secret.
fn raw_ref_secret_ty(ty: Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Str => Some("&str"),
        ty::Slice(element) if u8_ty(*element) => Some("&[u8]"),
        _ => None,
    }
}

/// Return type information for raw hir secret.
fn raw_hir_secret_ty(ty: &rustc_hir::Ty<'_>) -> Option<&'static str> {
    match ty.kind {
        rustc_hir::TyKind::Path(rustc_hir::QPath::Resolved(_, path)) => {
            raw_hir_path_secret_ty(path)
        }
        rustc_hir::TyKind::Ref(_, mut_ty) => raw_hir_ref_secret_ty(mut_ty.ty),
        rustc_hir::TyKind::Slice(element) if hir_u8_ty(element) => Some("[u8]"),
        rustc_hir::TyKind::Array(element, _) if hir_u8_ty(element) => Some("[u8; N]"),
        _ => None,
    }
}

/// Return the supported raw representation for a resolved HIR path.
fn raw_hir_path_secret_ty(path: &rustc_hir::Path<'_>) -> Option<&'static str> {
    path.segments
        .last()
        .filter(|segment| segment.ident.name.as_str() == "String")
        .map(|_| "String")
}

/// Return the supported raw representation for a referenced HIR type.
fn raw_hir_ref_secret_ty(ty: &rustc_hir::Ty<'_>) -> Option<&'static str> {
    match ty.kind {
        rustc_hir::TyKind::Path(rustc_hir::QPath::Resolved(_, path))
            if path
                .segments
                .last()
                .is_some_and(|segment| segment.ident.name.as_str() == "str") =>
        {
            Some("&str")
        }
        rustc_hir::TyKind::Slice(element) if hir_u8_ty(element) => Some("&[u8]"),
        _ => None,
    }
}

/// Return type information for slice u8.
fn slice_u8_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Slice(element) if u8_ty(*element))
}

/// Return type information for u8.
fn u8_ty(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Uint(ty::UintTy::U8))
}

/// Return type information for hir u8.
fn hir_u8_ty(ty: &rustc_hir::Ty<'_>) -> bool {
    let rustc_hir::TyKind::Path(rustc_hir::QPath::Resolved(_, path)) = ty.kind else {
        return false;
    };

    path.segments
        .last()
        .is_some_and(|segment| segment.ident.name.to_ident_string() == "u8")
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
