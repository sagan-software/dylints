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
    Body, FieldDef, FnDecl, LangItem, LetStmt, LocalSource, Param, Pat, PatKind, TraitFn,
    TraitItem, TraitItemKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Ident, Span, def_id::LocalDefId, sym};

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
        local_def_id: LocalDefId,
    ) {
        // Closures have no named boundary, and trait impls inherit their signature from the trait.
        if matches!(kind, FnKind::Closure) || implements_trait_item(cx, local_def_id) {
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
        let TraitItemKind::Fn(_sig, TraitFn::Required(arg_names)) = item.kind else {
            return;
        };

        check_trait_required_params(cx, arg_names, item.owner_id.def_id);
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

/// Check the named parameters of a required trait method against its resolved signature.
fn check_trait_required_params(
    cx: &LateContext<'_>,
    arg_names: &[Option<Ident>],
    local_def_id: LocalDefId,
) {
    // Required methods have no body type table, so read the resolved signature instead.
    let sig_inputs = cx
        .tcx
        .fn_sig(local_def_id)
        .instantiate_identity()
        .skip_binder()
        .inputs();
    // Pair source names with resolved types before checking secret vocabulary.
    for (maybe_ident, param_ty) in arg_names.iter().zip(sig_inputs) {
        let Some(ident) = maybe_ident else {
            continue;
        };
        let name = ident.name.to_ident_string();
        if secret_name(&name) {
            check_named_ty(cx, "parameter", ident.span, &name, *param_ty);
        }
    }
}

/// Return whether a function implements an item of a trait impl.
fn implements_trait_item(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    cx.tcx
        .impl_of_assoc(local_def_id.to_def_id())
        .is_some_and(|impl_def_id| cx.tcx.impl_opt_trait_id(impl_def_id).is_some())
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
    // Missing generic arguments stay outside the supported raw representations.
    let first_arg = args.iter().next().and_then(ty::GenericArg::as_type);
    if cx.tcx.is_lang_item(adt.did(), LangItem::String) {
        Some("String")
    } else if cx.tcx.is_diagnostic_item(sym::Vec, adt.did()) && first_arg.is_some_and(u8_ty) {
        Some("Vec<u8>")
    } else if adt.is_box() && first_arg.is_some_and(slice_u8_ty) {
        Some("Box<[u8]>")
    } else {
        None
    }
}

/// Return type information for raw ref secret.
fn raw_ref_secret_ty(ty: Ty<'_>) -> Option<&'static str> {
    match ty.kind() {
        ty::Str => Some("&str"),
        ty::Slice(element) if u8_ty(*element) => Some("&[u8]"),
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

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
