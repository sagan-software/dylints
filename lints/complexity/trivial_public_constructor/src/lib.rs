#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc resolution variants"
)]

//! A lint to check for trivial field-forwarding constructors.
//!
//! It resolves public `new` methods that only return a struct literal populated
//! by shorthand parameters. The lint requires a side-effect-free body, a complete
//! field list, and public fields before recommending direct construction to callers.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Body, Expr, ExprField, ExprKind, FnDecl, ImplicitSelfKind, QPath, StructTailExpr, def::Res,
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty::{AdtDef, Ty, TyCtxt};
use rustc_span::{
    Span,
    def_id::{DefId, LocalDefId},
};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TRIVIAL_PUBLIC_CONSTRUCTOR,
    Warn,
    "trivial field-forwarding constructor",
    TrivialPublicConstructor
}

impl<'tcx> LateLintPass<'tcx> for TrivialPublicConstructor {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        decl: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Restrict the public API check to methods with a stable method name.
        let FnKind::Method(ident, _) = kind else {
            return;
        };

        // Establish the constructor signature before inspecting its returned literal.
        let method_is_new = ident.name.to_ident_string() == "new";
        let has_no_self = decl.implicit_self() == ImplicitSelfKind::None;
        let has_inputs = !decl.inputs.is_empty();
        let is_struct_forward =
            has_inputs && trivial_struct_forward(cx, body.value, decl.inputs.len(), local_def_id);
        if method_is_new && has_no_self && has_inputs && is_struct_forward {
            emit_span_lint_with_help(
                cx,
                TRIVIAL_PUBLIC_CONSTRUCTOR,
                span,
                "`new` only forwards fields into a struct literal",
                "remove the constructor when public fields already express the same construction",
            );
        }
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for trivial struct forward analysis.
fn trivial_struct_forward<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    input_count: usize,
    constructor_def_id: LocalDefId,
) -> bool {
    // Keep each source-shape check in a small option stage so this predicate has one exit path.
    block_tail_expr(expr)
        .and_then(|final_expr| {
            forwarded_struct_literal(final_expr, input_count).and_then(|(qpath, fields)| {
                struct_def_id(cx, qpath, final_expr).map(|struct_def_id| (fields, struct_def_id))
            })
        })
        .filter(|(_, struct_def_id)| {
            returns_constructed_type(cx, constructor_def_id, *struct_def_id)
        })
        .is_some_and(|(fields, struct_def_id)| {
            let adt = cx.tcx.adt_def(struct_def_id);
            // Public fields make direct struct construction available to callers.
            fields
                .iter()
                .all(|field| field_is_public(cx.tcx, adt, field.ident.name))
        })
}

/// Return the side-effect-free tail expression of a constructor body.
const fn block_tail_expr<'tcx>(expr: &'tcx Expr<'tcx>) -> Option<&'tcx Expr<'tcx>> {
    match expr.kind {
        ExprKind::Block(block, _) if block.stmts.is_empty() => block.expr,
        _ => None,
    }
}

/// Return a complete shorthand struct literal with the expected input count.
fn forwarded_struct_literal<'tcx>(
    expr: &'tcx Expr<'tcx>,
    input_count: usize,
) -> Option<(&'tcx QPath<'tcx>, &'tcx [ExprField<'tcx>])> {
    match expr.kind {
        ExprKind::Struct(qpath, fields, StructTailExpr::None)
            if fields.len() == input_count && fields.iter().all(|field| field.is_shorthand) =>
        {
            Some((qpath, fields))
        }
        _ => None,
    }
}

/// Helper for struct def id analysis.
fn struct_def_id<'tcx>(
    cx: &LateContext<'tcx>,
    qpath: &'tcx QPath<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<DefId> {
    if let Some(adt) = cx.typeck_results().expr_ty(expr).ty_adt_def() {
        return Some(adt.did());
    }

    match cx.typeck_results().qpath_res(qpath, expr.hir_id) {
        Res::Def(_, def_id) => Some(def_id),
        _ => None,
    }
}

/// Return whether the item returns constructed type.
fn returns_constructed_type(
    cx: &LateContext<'_>,
    constructor_def_id: LocalDefId,
    struct_def_id: DefId,
) -> bool {
    let output = fn_output_ty(cx.tcx, constructor_def_id);
    output
        .ty_adt_def()
        .is_some_and(|adt| adt.did() == struct_def_id)
}

/// Helper for field is public analysis.
fn field_is_public(tcx: TyCtxt<'_>, adt: AdtDef<'_>, field_name: rustc_span::Symbol) -> bool {
    adt.all_fields()
        .find(|field| field.name == field_name)
        .is_some_and(|field| tcx.visibility(field.did).is_public())
}

/// Return type information for fn output.
fn fn_output_ty(tcx: TyCtxt<'_>, local_def_id: LocalDefId) -> Ty<'_> {
    tcx.fn_sig(local_def_id)
        .instantiate_identity()
        .skip_norm_wip()
        .output()
        .skip_binder()
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
