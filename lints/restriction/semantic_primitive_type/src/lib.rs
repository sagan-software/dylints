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

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprKind, FieldDef, FnDecl, FnRetTy, Param, PatKind, intravisit::FnKind,
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
        // Ignore closures because their inferred boundary is not a named API.
        if matches!(kind, FnKind::Closure) {
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

        // Check an explicit return type against the enclosing function name.
        let FnRetTy::Return(output) = decl.output else {
            return;
        };
        let Some(name) = function_name(kind) else {
            return;
        };
        let output_ty = cx
            .tcx
            .fn_sig(local_def_id)
            .instantiate_identity()
            .skip_norm_wip()
            .output()
            .skip_binder();
        check_named_type(cx, "return value", &name, output.span, output_ty);
    }

    /// Check semantic string matches that encode a closed vocabulary.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Restrict source analysis to matches over resolved string values.
        let ExprKind::Match(scrutinee, _, _) = expr.kind else {
            return;
        };
        if !is_string(
            cx,
            peel_wrappers(cx, cx.typeck_results().expr_ty(scrutinee)),
        ) {
            return;
        }

        // Count literal arms and require an explicit catch-all fallback.
        let Ok(source) = cx.sess().source_map().span_to_snippet(expr.span) else {
            return;
        };
        let literal_arms = source
            .lines()
            .filter(|line| line.trim_start().starts_with('"') && line.contains("=>"))
            .count();
        let has_catch_all = source
            .lines()
            .any(|line| line.trim_start().starts_with("_ =>"));
        if literal_arms >= 3 && has_catch_all {
            emit_lint(
                cx,
                expr.span,
                "string match encodes a closed vocabulary with primitive strings",
                "parse the input into a closed enum and match on its variants",
            );
        }
    }
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
    // Pair recognized boundary names with their disallowed primitive families.
    let finding = if identity_name(name) && is_integer(base) {
        Some(("identity data", primitive_name(base)))
    } else if http_status_name(name) && is_integer(base) {
        Some(("an HTTP status", primitive_name(base)))
    } else if reason_name(name) && is_string(cx, base) {
        Some(("a reason code", primitive_name(base)))
    } else {
        None
    };
    let Some((domain, primitive)) = finding else {
        return;
    };

    // Report the primitive and prescribe conversion at the system boundary.
    emit_lint(
        cx,
        span,
        format!("{label} `{name}` uses primitive type `{primitive}` for {domain}"),
        "introduce a validated semantic type and convert at the system boundary",
    );
}

/// Peel transparent containers before classifying the stored domain value.
fn peel_wrappers<'tcx>(cx: &LateContext<'tcx>, mut ty: Ty<'tcx>) -> Ty<'tcx> {
    loop {
        ty = match ty.kind() {
            ty::Ref(_, inner, _) | ty::Slice(inner) | ty::Array(inner, _) => *inner,
            ty::Adt(adt, args)
                if cx.tcx.is_diagnostic_item(sym::Option, adt.did())
                    || cx.tcx.item_name(adt.did()).as_str() == "Vec" =>
            {
                args.type_at(0)
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
        || matches!(ty.kind(), ty::Adt(adt, _) if cx.tcx.item_name(adt.did()) == sym::String)
}

/// Render the semantic primitive for the diagnostic.
fn primitive_name(ty: Ty<'_>) -> String {
    ty.to_string()
        .replace("std::string::", "")
        .replace("alloc::string::", "")
}

/// Return whether the name denotes an identity.
fn identity_name(name: &str) -> bool {
    name.ends_with("_id")
}

/// Return whether the name denotes an HTTP status.
fn http_status_name(name: &str) -> bool {
    name == "http_status" || name.ends_with("_http_status")
}

/// Return whether the name denotes a closed reason code.
fn reason_name(name: &str) -> bool {
    matches!(name, "reason" | "reasons" | "reason_code" | "reason_codes")
        || name.ends_with("_reason_code")
        || name.ends_with("_reason_codes")
}

/// Return a simple parameter name.
fn binding_name(param: &Param<'_>) -> Option<String> {
    let PatKind::Binding(_, _, ident, None) = param.pat.kind else {
        return None;
    };
    Some(ident.name.to_ident_string())
}

/// Return a free-function or method name.
fn function_name(kind: FnKind<'_>) -> Option<String> {
    match kind {
        FnKind::ItemFn(ident, ..) | FnKind::Method(ident, ..) => Some(ident.name.to_ident_string()),
        FnKind::Closure => None,
    }
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
