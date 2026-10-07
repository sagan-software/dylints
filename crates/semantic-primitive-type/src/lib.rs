#![feature(rustc_private)]
#![expect(
    clippy::wildcard_enum_match_arm,
    clippy::let_underscore_must_use,
    reason = "only supported primitive wrappers are relevant and diagnostics are configured in place"
)]
#![warn(unused_extern_crates)]

//! A lint to check for domain values stored as primitive types.
//!
//! It checks named fields, parameters, return values, and closed string matches
//! for identities, status codes, reason values, and similar semantic domains.
//! The diagnostic keeps parsing and serialization at the boundary while asking
//! callers to retain a validated type for internal operations and invariants.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Arm, Body, Expr, ExprKind, FieldDef, FnDecl, FnRetTy, LangItem, Param, Pat, PatExprKind,
    PatKind, TraitFn, TraitItem, TraitItemKind, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SEMANTIC_PRIMITIVE_TYPE,
    Warn,
    "domain value stored as a primitive type",
    SemanticPrimitiveType
}

impl<'tcx> LateLintPass<'tcx> for SemanticPrimitiveType {
    /// Check named fields for raw domain values.
    fn check_field_def(&mut self, cx: &LateContext<'tcx>, field: &'tcx FieldDef<'tcx>) {
        check_named_type(
            cx,
            "field",
            field.ident.name.as_str(),
            field.ty.span,
            cx.tcx
                .type_of(field.def_id)
                .instantiate_identity()
                .skip_norm_wip(),
        );
    }

    /// Check named parameters and function return values.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Closures have no named boundary, and trait impls inherit their signature from the trait.
        let (FnKind::ItemFn(ident, ..) | FnKind::Method(ident, ..)) = kind else {
            return;
        };
        if cx
            .tcx
            .impl_of_assoc(local_def_id.to_def_id())
            .is_some_and(|impl_def_id| cx.tcx.impl_opt_trait_id(impl_def_id).is_some())
        {
            return;
        }

        // Align each source parameter type with its direct binding name.
        for (source_ty, param) in decl.inputs.iter().zip(body.params) {
            let Some(name) = binding_name(param) else {
                continue;
            };
            check_named_type(
                cx,
                "parameter",
                &name,
                source_ty.span,
                cx.typeck_results().node_type(param.hir_id),
            );
        }

        check_return(cx, decl, local_def_id, ident.name.as_str());
    }

    /// Check the parameters and return value of a required trait method.
    fn check_trait_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx TraitItem<'tcx>) {
        let TraitItemKind::Fn(sig, TraitFn::Required(arg_names)) = item.kind else {
            return;
        };
        let local_def_id = item.owner_id.def_id;

        // Required methods have no body type table, so read the resolved signature instead.
        // Pair source names with resolved types before checking the return value.
        let sig_inputs = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_binder()
            .inputs();
        // Check named parameters before the return value so diagnostics follow source order.
        sig.decl
            .inputs
            .iter()
            .zip(arg_names)
            .zip(sig_inputs)
            .filter_map(|((source_ty, maybe_ident), param_ty)| {
                maybe_ident.map(|ident| (ident, source_ty.span, *param_ty))
            })
            .for_each(|(ident, span, param_ty)| {
                check_named_type(cx, "parameter", ident.name.as_str(), span, param_ty);
            });
        check_return(cx, sig.decl, local_def_id, item.ident.name.as_str());
    }

    /// Check semantic string matches that encode a closed vocabulary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Restrict analysis to source-written matches over resolved string values.
        let ExprKind::Match(scrutinee, arms, _) = expr.kind else {
            return;
        };
        if expr.span.from_expansion()
            || !is_string(
                cx,
                peel_wrappers(cx, cx.typeck_results().expr_ty(scrutinee)),
            )
        {
            return;
        }

        // Count string-literal arms and require an unguarded catch-all fallback.
        let literal_arms = arms
            .iter()
            .filter(|arm| arm.guard.is_none() && is_string_literal_pattern(arm.pat))
            .count();
        if literal_arms < 3 || !arms.iter().any(is_catch_all_arm) {
            return;
        }

        // Parsing strings inside `FromStr` or `TryFrom` is the boundary this lint recommends.
        if in_parsing_impl(cx, expr) {
            return;
        }
        emit_lint(
            cx,
            expr.span,
            "string match encodes a closed vocabulary with primitive strings",
            "parse the input into a closed enum and match on its variants",
        );
    }
}

/// Return whether a pattern matches only string literals, including `"a" | "b"`.
fn is_string_literal_pattern(pat: &Pat<'_>) -> bool {
    match pat.kind {
        PatKind::Expr(pat_expr) => matches!(
            pat_expr.kind,
            PatExprKind::Lit { lit, .. } if matches!(lit.node, LitKind::Str(..))
        ),
        PatKind::Or(alternatives) => alternatives.iter().all(is_string_literal_pattern),
        _ => false,
    }
}

/// Return whether an arm accepts every remaining value with `_` or a plain binding.
const fn is_catch_all_arm(arm: &Arm<'_>) -> bool {
    arm.guard.is_none() && matches!(arm.pat.kind, PatKind::Wild | PatKind::Binding(.., None))
}

/// Return whether an expression is inside `FromStr::from_str` or `TryFrom::try_from`.
fn in_parsing_impl(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Closures inside the parser share its typeck root, so resolve the outermost body owner.
    let owner = cx.tcx.hir_enclosing_body_owner(expr.hir_id);
    let root = cx.tcx.typeck_root_def_id(owner.to_def_id());
    let Some(trait_item) = cx.tcx.trait_item_of(root) else {
        return false;
    };
    // `FromStr` has no diagnostic item, but its `from_str` method does.
    cx.tcx
        .get_diagnostic_name(trait_item)
        .is_some_and(|name| name.as_str() == "from_str_method")
        || cx
            .tcx
            .trait_of_assoc(trait_item)
            .is_some_and(|trait_def_id| cx.tcx.is_diagnostic_item(sym::TryFrom, trait_def_id))
}

/// Check an explicit return type against the function name.
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
    check_named_type(cx, "return value", name, output.span, output_ty);
}

/// Check one named semantic boundary.
fn check_named_type<'tcx>(
    cx: &LateContext<'tcx>,
    label: &str,
    name: &str,
    span: Span,
    ty: Ty<'tcx>,
) {
    // Peel transparent wrappers before classifying the semantic value.
    let base = peel_wrappers(cx, ty);
    let Some(domain) = semantic_domain(cx, name, base) else {
        return;
    };
    let primitive = base
        .to_string()
        .replace("std::string::", "")
        .replace("alloc::string::", "");

    // Report the primitive and prescribe conversion at the system boundary.
    emit_lint(
        cx,
        span,
        format!("{label} `{name}` uses primitive type `{primitive}` for {domain}"),
        "introduce a validated semantic type and convert at the system boundary",
    );
}

/// Return the semantic domain when a boundary name and primitive type match.
fn semantic_domain(cx: &LateContext<'_>, name: &str, base: Ty<'_>) -> Option<&'static str> {
    // Integer domains use names that identify either an object or an HTTP status.
    if is_integer(base) {
        if is_identity_name(name) {
            return Some("identity data");
        }
        if is_http_status_name(name) {
            return Some("an HTTP status");
        }
    }
    // String domains use closed names for machine-readable reason codes.
    if is_string(cx, base) && is_reason_name(name) {
        return Some("a reason code");
    }
    None
}

/// Return whether a name identifies an object or resource.
fn is_identity_name(name: &str) -> bool {
    name.ends_with("_id")
}

/// Return whether a name identifies an HTTP status value.
fn is_http_status_name(name: &str) -> bool {
    name == "http_status" || name.ends_with("_http_status")
}

/// Return whether a name identifies a machine-readable reason code.
fn is_reason_name(name: &str) -> bool {
    matches!(name, "reason" | "reasons" | "reason_code" | "reason_codes")
        || name.ends_with("_reason_code")
        || name.ends_with("_reason_codes")
}

/// Peel transparent containers before classifying the stored domain value.
fn peel_wrappers<'tcx>(cx: &LateContext<'tcx>, mut ty: Ty<'tcx>) -> Ty<'tcx> {
    loop {
        // Remove transparent wrappers until the stored semantic primitive is visible.
        ty = match ty.kind() {
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => *inner,
            ty::Adt(adt, _) if cx.tcx.is_diagnostic_item(sym::Option, adt.did()) => {
                // Preserve an over-depth standard Option chain as an opaque type.
                let Some(inner) = dylint_support::peel_standard_options(cx.tcx, ty) else {
                    return ty;
                };
                inner
            }
            ty::Adt(adt, args) if cx.tcx.is_diagnostic_item(sym::Vec, adt.did()) => {
                let Some(inner) = args.types().next() else {
                    return ty;
                };
                inner
            }
            _ => return ty,
        };
    }
}

/// Return whether the primitive is an integer.
fn is_integer(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Int(_) | ty::Uint(_))
}

/// Return whether the primitive is `str` or `String`.
fn is_string(cx: &LateContext<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Str)
        || matches!(ty.kind(), ty::Adt(adt, _) if cx.tcx.is_lang_item(adt.did(), LangItem::String))
}

/// Return a simple parameter name.
fn binding_name(param: &Param<'_>) -> Option<String> {
    let PatKind::Binding(_, _, ident, None) = param.pat.kind else {
        return None;
    };
    Some(ident.name.to_ident_string())
}

/// Emit one semantic primitive diagnostic.
fn emit_lint(cx: &LateContext<'_>, span: Span, message: impl Into<String>, help: &'static str) {
    let message = message.into();
    cx.emit_span_lint(
        SEMANTIC_PRIMITIVE_TYPE,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
