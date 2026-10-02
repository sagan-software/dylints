#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc resolution variants"
)]

//! A lint to check for trivial field-forwarding constructors.
//!
//! It resolves public inherent `new` methods that only return a literal of a
//! public, exhaustive struct whose fields are public and populated by distinct
//! parameters. Callers can then build the same value with a struct literal.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprField, ExprKind, FnDecl, HirId, ImplicitSelfKind, PatKind, StructTailExpr,
    def::Res, intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};

use rustc_span::{Span, def_id::LocalDefId, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRIVIAL_PUBLIC_CONSTRUCTOR,
    Warn,
    "trivial field-forwarding constructor",
    TrivialPublicConstructor
}

impl<'tcx> LateLintPass<'tcx> for TrivialPublicConstructor {
    /// Checks public inherent `new` methods that only forward their parameters.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Restrict the check to a user-written associated `new` without `self`.
        let FnKind::Method(ident, _) = kind else {
            return;
        };
        let def_id = local_def_id.to_def_id();
        let is_new_without_self = ident.name == sym::new
            && decl.implicit_self() == ImplicitSelfKind::None
            && !decl.inputs.is_empty();

        // Trait methods keep their signature, and only a public method is public API.
        let is_public_inherent = cx.tcx.trait_of_assoc(def_id).is_none()
            && cx.tcx.trait_impl_of_assoc(def_id).is_none()
            && cx.tcx.visibility(def_id).is_public();
        if is_new_without_self
            && is_public_inherent
            && !span.from_expansion()
            && trivial_struct_forward(cx, body)
        {
            cx.emit_span_lint(
                TRIVIAL_PUBLIC_CONSTRUCTOR,
                span,
                DiagDecorator(|diag| {
                    let _ = diag.primary_message("`new` only forwards fields into a struct literal");
                    let _ = diag.help(
                        "remove the constructor when public fields already express the same construction",
                    );
                }),
            );
        }
    }
}

/// Returns whether the body only moves each parameter into a public field.
fn trivial_struct_forward<'tcx>(cx: &LateContext<'tcx>, body: &'tcx Body<'tcx>) -> bool {
    // Recover the returned literal and the struct it builds.
    let Some((fields, struct_def_id)) = block_tail_expr(body.value)
        .and_then(|literal| forwarded_struct_literal(literal).map(|fields| (literal, fields)))
        .and_then(|(literal, fields)| {
            cx.typeck_results()
                .expr_ty(literal)
                .ty_adt_def()
                .map(|adt| (fields, adt.did()))
        })
    else {
        return false;
    };

    // Other crates cannot use a literal for a private or non-exhaustive struct.
    let adt = cx.tcx.adt_def(struct_def_id);
    let is_constructible = adt.is_struct()
        && cx.tcx.visibility(struct_def_id).is_public()
        && !adt.non_enum_variant().is_field_list_non_exhaustive();

    // A literal without `..` names every field, so every field must be public.
    is_constructible
        && is_parameter_forwarding(cx, body, fields)
        && adt
            .all_fields()
            .all(|field| cx.tcx.visibility(field.did).is_public())
}

/// Returns the tail expression of a statement-free constructor body.
const fn block_tail_expr<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match expr.kind {
        ExprKind::Block(block, _) if block.stmts.is_empty() => block.expr,
        _ => None,
    }
}

/// Returns the fields of a complete shorthand struct literal.
fn forwarded_struct_literal<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx [ExprField<'tcx>]> {
    match expr.kind {
        ExprKind::Struct(_, fields, StructTailExpr::None)
            if fields.iter().all(|field| field.is_shorthand) =>
        {
            Some(fields)
        }
        _ => None,
    }
}

/// Returns whether the fields consume every parameter binding exactly once.
fn is_parameter_forwarding(
    cx: &LateContext<'_>,
    body: &Body<'_>,
    fields: &[ExprField<'_>],
) -> bool {
    // A shorthand field can also name a constant, so resolve every value to a parameter.
    let parameters = body
        .params
        .iter()
        .filter_map(|param| match param.pat.kind {
            PatKind::Binding(_, hir_id, _, None) => Some(hir_id),
            _ => None,
        })
        .collect::<Vec<HirId>>();
    parameters.len() == body.params.len()
        && parameters.len() == fields.len()
        && fields.iter().all(|field| {
            matches!(
                field.expr.kind,
                ExprKind::Path(ref qpath)
                    if matches!(
                        cx.qpath_res(qpath, field.expr.hir_id),
                        Res::Local(hir_id) if parameters.contains(&hir_id)
                    )
            )
        })
}

/// Runs the UI fixtures.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
