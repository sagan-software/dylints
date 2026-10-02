#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores diagnostic builders and unrelated rustc syntax variants"
)]

//! A lint to check for owned inputs whose fields are only cloned into outputs.
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
    Body, Expr, ExprField, ExprKind, HirId, Param, Pat, PatKind, StructTailExpr, def::Res,
    intravisit::FnKind,
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, def_id::LocalDefId, kw};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub OWNED_INPUT_FIELD_CLONES,
    Warn,
    "owned input is only borrowed to clone fields into an output",
    OwnedInputFieldClones
}

impl<'tcx> LateLintPass<'tcx> for OwnedInputFieldClones {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _span: Span,
        local_def_id: LocalDefId,
    ) {
        // Analyze named functions and methods because closures have no stable API boundary.
        if matches!(kind, FnKind::Closure) {
            return;
        }
        // A trait fixes the parameter types of its methods and their implementations.
        let has_fixed_signature = is_trait_method(cx, local_def_id);

        // Restrict the output shape to a returned struct literal.
        let Some((fields, tail)) = returned_struct_literal(body.value) else {
            return;
        };
        // Skip update syntax because unlisted fields can consume the parameter.
        if !matches!(tail, StructTailExpr::None) {
            return;
        }

        // Evaluate clone and move origins independently for each simple parameter.
        for param in simple_params(cx, body) {
            // Both rules need at least two fields cloned from the parameter.
            if cloned_field_count(cx, fields, param.hir_id) < 2 {
                continue;
            }

            // Report a borrowed input only when the signature could take ownership instead.
            if param.is_borrowed {
                let can_take_ownership = !has_fixed_signature && param.name != kw::SelfLower;
                if can_take_ownership {
                    emit_span_lint_with_help(
                        cx,
                        OWNED_INPUT_FIELD_CLONES,
                        param.span,
                        format!(
                            "borrowed parameter `{}` is cloned into the returned struct",
                            param.name
                        ),
                        "take ownership when the output must own data copied from the input",
                    );
                }
                continue;
            }

            // Report an owned input only when no field already moves out of it.
            let has_moved_field = fields
                .iter()
                .any(|field| direct_move_from_param(cx, field.expr, param.hir_id));
            if !has_moved_field {
                emit_span_lint_with_help(
                    cx,
                    OWNED_INPUT_FIELD_CLONES,
                    param.span,
                    format!(
                        "owned parameter `{}` is only borrowed to clone fields into the returned struct",
                        param.name
                    ),
                    "consume the input by destructuring it, or take it by reference if cloning is intentional",
                );
            }
        }
    }
}

/// Return whether a trait declares or implements this function's signature.
fn is_trait_method(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let def_id = local_def_id.to_def_id();
    cx.tcx.trait_of_assoc(def_id).is_some() || cx.tcx.trait_impl_of_assoc(def_id).is_some()
}

/// State used by the owned param analysis.
struct OwnedParam {
    /// hir id stored for this lint's analysis.
    hir_id: HirId,
    /// name stored for this lint's analysis.
    name: Symbol,
    /// span stored for this lint's analysis.
    span: Span,
    /// Whether the caller retains ownership of the parameter.
    is_borrowed: bool,
}

/// Return simple parameters whose field origins support tracking.
fn simple_params(cx: &LateContext<'_>, body: &Body<'_>) -> Vec<OwnedParam> {
    body.params
        .iter()
        .filter_map(|param| simple_param(cx, param))
        .collect()
}

/// Resolve one simple parameter and record its ownership mode.
fn simple_param(cx: &LateContext<'_>, param: &Param<'_>) -> Option<OwnedParam> {
    let is_borrowed = matches!(
        cx.typeck_results().node_type(param.hir_id).kind(),
        ty::Ref(..)
    );

    // Destructured parameters already consume the input explicitly.
    let (name, hir_id) = binding(param.pat)?;

    Some(OwnedParam {
        hir_id,
        name,
        span: param.pat.span,
        is_borrowed,
    })
}

/// Helper for returned struct literal analysis.
fn returned_struct_literal<'hir>(
    expr: &'hir Expr<'hir>,
) -> Option<(&'hir [ExprField<'hir>], StructTailExpr<'hir>)> {
    match tail_expr(expr).kind {
        ExprKind::Struct(_, fields, tail) => Some((fields, tail)),
        _ => None,
    }
}

/// Helper for tail expr analysis.
fn tail_expr<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    let ExprKind::Block(block, _) = expr.kind else {
        return expr;
    };

    // Only the semicolon-free final expression is the returned value.
    block.expr.map_or(expr, tail_expr)
}

/// Helper for clones multiple fields analysis.
fn cloned_field_count(
    cx: &LateContext<'_>,
    fields: &[ExprField<'_>],
    param_hir_id: HirId,
) -> usize {
    fields
        .iter()
        .filter_map(|field| cloned_field_from_param(cx, field.expr, param_hir_id))
        .count()
}

/// Helper for cloned field from param analysis.
fn cloned_field_from_param(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    param_hir_id: HirId,
) -> Option<Symbol> {
    // Require a zero-argument method call whose written name is `clone`.
    let ExprKind::MethodCall(segment, receiver, [], _) = expr.kind else {
        return None;
    };
    if segment.ident.name.as_str() != "clone" {
        return None;
    }
    if !resolved_clone_trait_method(cx, expr) {
        return None;
    }

    // Trace the receiver back to a field of the selected parameter.
    field_from_param(cx, receiver, param_hir_id)
}

/// Helper for direct move from param analysis.
fn direct_move_from_param(cx: &LateContext<'_>, expr: &Expr<'_>, param_hir_id: HirId) -> bool {
    if cloned_field_from_param(cx, expr, param_hir_id).is_some() {
        return false;
    }

    path_is_binding(cx, expr, param_hir_id) || field_from_param(cx, expr, param_hir_id).is_some()
}

/// Helper for field from param analysis.
fn field_from_param(cx: &LateContext<'_>, expr: &Expr<'_>, param_hir_id: HirId) -> Option<Symbol> {
    let ExprKind::Field(receiver, field) = expr.kind else {
        return None;
    };

    path_is_binding(cx, receiver, param_hir_id).then_some(field.name)
}

/// Return whether resolution found clone trait method.
fn resolved_clone_trait_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve the called method instead of trusting its source spelling.
    cx.typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .is_some_and(|def_id| {
            let method_path = cx.tcx.def_path_str(def_id);
            // Accept direct and qualified paths emitted for the standard Clone trait.
            let has_canonical_path = matches!(
                method_path.as_str(),
                "core::clone::Clone::clone" | "std::clone::Clone::clone"
            ) || method_path.ends_with(" as core::clone::Clone>::clone")
                || method_path.ends_with(" as std::clone::Clone>::clone");
            has_canonical_path
                || cx
                    .tcx
                    .opt_associated_item(def_id)
                    .filter(|assoc_item| assoc_item.name().as_str() == "clone")
                    .and_then(|assoc_item| assoc_item.trait_item_or_self().ok())
                    .and_then(|trait_item| cx.tcx.trait_of_assoc(trait_item))
                    .is_some_and(|trait_def_id| {
                        matches!(
                            cx.tcx.def_path_str(trait_def_id).as_str(),
                            "core::clone::Clone" | "std::clone::Clone"
                        )
                    })
        })
}

/// Helper for binding analysis.
const fn binding(pat: &Pat<'_>) -> Option<(Symbol, HirId)> {
    let PatKind::Binding(_mode, hir_id, ident, None) = pat.kind else {
        return None;
    };

    Some((ident.name, hir_id))
}

/// Helper for path is binding analysis.
fn path_is_binding(cx: &LateContext<'_>, expr: &Expr<'_>, hir_id: HirId) -> bool {
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };

    matches!(
        cx.typeck_results().qpath_res(&qpath, expr.hir_id),
        Res::Local(resolved_hir_id) if resolved_hir_id == hir_id
    )
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

    // Use rustc's native diagnostic decorator to keep this lint Clippy-independent.
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
