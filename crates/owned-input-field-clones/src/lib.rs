#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores diagnostic builders after configuring them"
)]

//! A lint to check for owned inputs whose fields are only cloned into outputs.
//!
//! It inspects a function's tail struct literal, resolves each `.clone()` call
//! to `Clone::clone`, and traces its receiver to a field of a by-value
//! parameter. Owned inputs whose fields are only cloned are reported with a
//! machine-applicable fix that moves the fields when the move provably
//! compiles; borrowed inputs are reported when the signature could take
//! ownership instead.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::ops::ControlFlow;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    BindingMode, Body, ByRef, Expr, ExprField, ExprKind, HirId, Param, PatKind, StructTailExpr,
    def::Res,
    intravisit::{FnKind, Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_middle::{
    hir::nested_filter::OnlyBodies,
    ty::{self, TyCtxt},
};
use rustc_span::{Span, Symbol, def_id::LocalDefId, kw, sym};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub OWNED_INPUT_FIELD_CLONES,
    Warn,
    "owned input is only borrowed to clone fields into an output",
    OwnedInputFieldClones
}

impl<'tcx> LateLintPass<'tcx> for OwnedInputFieldClones {
    /// Check a named function's tail struct literal for cloned parameter fields.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Analyze written named functions and methods; closures have no stable API boundary.
        if matches!(kind, FnKind::Closure) || span.from_expansion() {
            return;
        }

        // Restrict the output shape to a returned struct literal without update syntax,
        // because unlisted fields could consume the parameter.
        let ExprKind::Struct(_, fields, StructTailExpr::None) = tail_expr(body.value).kind else {
            return;
        };
        let literal = StructLiteral {
            fields,
            is_whole_body: is_whole_body(body.value),
            has_fixed_signature: has_fixed_signature(cx, local_def_id),
        };

        // Evaluate clone and move origins independently for each simple parameter.
        for param in body
            .params
            .iter()
            .filter_map(|param| simple_param(cx, param))
        {
            check_param(cx, &literal, &param);
        }
    }
}

/// The returned struct literal and the facts about its function that the rules need.
struct StructLiteral<'tcx> {
    /// Field initializers of the literal.
    fields: &'tcx [ExprField<'tcx>],
    /// Whether the function body is only the literal, with no statements before it.
    is_whole_body: bool,
    /// Whether a trait declares or implements the function's signature.
    has_fixed_signature: bool,
}

/// Return whether a trait declares or implements this function's signature.
fn has_fixed_signature(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    let def_id = local_def_id.to_def_id();
    let is_trait_item = cx.tcx.trait_of_assoc(def_id).is_some();
    let is_trait_impl_item = cx.tcx.trait_impl_of_assoc(def_id).is_some();

    is_trait_item || is_trait_impl_item
}

/// Apply the borrowed or owned rule to one parameter of the function.
fn check_param<'tcx>(cx: &LateContext<'tcx>, literal: &StructLiteral<'tcx>, param: &SimpleParam) {
    let clones: Vec<FieldClone<'_>> = literal
        .fields
        .iter()
        .filter_map(|field| field_clone(cx, field.expr, param.hir_id))
        .collect();
    // Both rules need at least two non-`Copy` fields cloned from the parameter.
    if clones.len() < 2 {
        return;
    }

    // Report a borrowed input only when the signature could take ownership instead.
    if param.is_borrowed {
        if !literal.has_fixed_signature && param.name != kw::SelfLower {
            emit_borrowed(cx, param);
        }
        return;
    }

    // Report an owned input only when no field already moves out of it and its type
    // allows moving fields out, which a `Drop` impl forbids.
    let has_moved_field = literal
        .fields
        .iter()
        .any(|field| is_direct_move_from_param(cx, field.expr, param.hir_id));
    if has_moved_field || !param.can_move_fields {
        return;
    }
    let is_fix_applicable =
        literal.is_whole_body && can_move_clones(cx, literal.fields, &clones, param.hir_id);
    emit_owned(cx, param, &clones, is_fix_applicable);
}

/// A simple by-value parameter binding whose field origins can be tracked.
struct SimpleParam {
    /// Binding introduced by the parameter pattern.
    hir_id: HirId,
    /// Name of the binding.
    name: Symbol,
    /// Span of the parameter pattern.
    span: Span,
    /// Whether the parameter type is a reference, so the caller keeps ownership.
    is_borrowed: bool,
    /// Whether the parameter is a struct without a `Drop` impl, so fields can move.
    can_move_fields: bool,
}

/// Resolve a by-value identifier parameter and record its ownership mode.
fn simple_param(cx: &LateContext<'_>, param: &Param<'_>) -> Option<SimpleParam> {
    // Destructured and `ref` parameters are not tracked.
    let PatKind::Binding(BindingMode(ByRef::No, _), hir_id, ident, None) = param.pat.kind else {
        return None;
    };
    let param_ty = cx.typeck_results().node_type(param.hir_id);
    let can_move_fields = matches!(param_ty.kind(), ty::Adt(adt, _)
        if adt.is_struct() && adt.destructor(cx.tcx).is_none());

    Some(SimpleParam {
        hir_id,
        name: ident.name,
        span: param.pat.span,
        is_borrowed: matches!(param_ty.kind(), ty::Ref(..)),
        can_move_fields,
    })
}

/// One `param.field.clone()` expression in the returned struct literal.
struct FieldClone<'tcx> {
    /// The complete `.clone()` call.
    call: &'tcx Expr<'tcx>,
    /// The `param.field` receiver.
    receiver: &'tcx Expr<'tcx>,
    /// The `param` path inside the receiver.
    base: &'tcx Expr<'tcx>,
    /// The cloned field name.
    field: Symbol,
}

/// Return the clone of a non-`Copy` parameter field, if the expression is one.
fn field_clone<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    param_hir_id: HirId,
) -> Option<FieldClone<'tcx>> {
    // Require a zero-argument method call that resolves to `Clone::clone`.
    let ExprKind::MethodCall(_segment, receiver, [], _) = expr.kind else {
        return None;
    };
    let is_clone = cx
        .typeck_results()
        .type_dependent_def_id(expr.hir_id)
        .and_then(|def_id| cx.tcx.trait_of_assoc(def_id))
        .is_some_and(|trait_def_id| cx.tcx.is_diagnostic_item(sym::Clone, trait_def_id));

    // Cloning a `Copy` field costs the same as moving it.
    let field_ty = cx.typeck_results().expr_ty(receiver);
    let is_copy = cx
        .tcx
        .type_is_copy_modulo_regions(cx.typing_env(), field_ty);

    // Trace the receiver back to a field of the selected parameter.
    let (base, field) =
        field_from_param(cx, receiver, param_hir_id).filter(|_| is_clone && !is_copy)?;
    Some(FieldClone {
        call: expr,
        receiver,
        base,
        field,
    })
}

/// Return whether the field expression moves the parameter or one of its fields.
fn is_direct_move_from_param<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    param_hir_id: HirId,
) -> bool {
    is_binding_path(cx, expr, param_hir_id) || field_from_param(cx, expr, param_hir_id).is_some()
}

/// Return the `param` path and field name when the expression is `param.field`.
fn field_from_param<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    param_hir_id: HirId,
) -> Option<(&'tcx Expr<'tcx>, Symbol)> {
    let ExprKind::Field(base, field) = expr.kind else {
        return None;
    };

    is_binding_path(cx, base, param_hir_id).then_some((base, field.name))
}

/// Return whether the expression is a path to the given local binding.
fn is_binding_path(cx: &LateContext<'_>, expr: &Expr<'_>, hir_id: HirId) -> bool {
    let ExprKind::Path(qpath) = expr.kind else {
        return false;
    };

    matches!(
        cx.typeck_results().qpath_res(&qpath, expr.hir_id),
        Res::Local(resolved_hir_id) if resolved_hir_id == hir_id
    )
}

/// Return the final expression of nested blocks.
fn tail_expr<'hir>(expr: &'hir Expr<'hir>) -> &'hir Expr<'hir> {
    let ExprKind::Block(block, _) = expr.kind else {
        return expr;
    };

    // Only the semicolon-free final expression is the returned value.
    block.expr.map_or(expr, tail_expr)
}

/// Return whether the body is only its tail expression, with no statements.
fn is_whole_body(expr: &Expr<'_>) -> bool {
    let ExprKind::Block(block, _) = expr.kind else {
        return true;
    };

    block.stmts.is_empty() && block.expr.is_some_and(is_whole_body)
}

/// Return whether replacing every clone with a field move provably compiles.
///
/// Each moved field must be read directly from the parameter value with the
/// cloned type and appear once. No other field expression may use the
/// parameter, because it would run after a field moved out.
fn can_move_clones<'tcx>(
    cx: &LateContext<'tcx>,
    fields: &'tcx [ExprField<'tcx>],
    clones: &[FieldClone<'tcx>],
    param_hir_id: HirId,
) -> bool {
    let has_only_direct_clones = clones.iter().all(|clone| is_direct_clone(cx, clone));
    let has_unique_fields = clones
        .iter()
        .enumerate()
        .all(|(index, clone)| is_first_clone_of_field(clones, index, clone));
    let has_other_use = fields
        .iter()
        .any(|field| is_other_param_use(cx, clones, field, param_hir_id));

    has_only_direct_clones && has_unique_fields && !has_other_use
}

/// Return whether moving the cloned field yields the same type without auto-deref.
fn is_direct_clone(cx: &LateContext<'_>, clone: &FieldClone<'_>) -> bool {
    // Auto-deref through a smart pointer would move out of a borrow.
    let typeck = cx.typeck_results();
    let has_no_deref = typeck.expr_adjustments(clone.base).is_empty();
    let has_same_type = typeck.expr_ty(clone.receiver) == typeck.expr_ty(clone.call);

    has_no_deref && has_same_type
}

/// Return whether no earlier clone in the literal clones the same field.
fn is_first_clone_of_field(
    clones: &[FieldClone<'_>],
    index: usize,
    clone: &FieldClone<'_>,
) -> bool {
    clones
        .iter()
        .take(index)
        .all(|earlier| earlier.field != clone.field)
}

/// Return whether a field initializer other than a counted clone uses the parameter.
fn is_other_param_use<'tcx>(
    cx: &LateContext<'tcx>,
    clones: &[FieldClone<'tcx>],
    field: &'tcx ExprField<'tcx>,
    param_hir_id: HirId,
) -> bool {
    let is_counted_clone = clones
        .iter()
        .any(|clone| clone.call.hir_id == field.expr.hir_id);

    !is_counted_clone && has_binding_use(cx, field.expr, param_hir_id)
}

/// Return whether the expression, including closures inside it, uses the binding.
fn has_binding_use<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>, hir_id: HirId) -> bool {
    BindingFinder { cx, hir_id }.visit_expr(expr).is_break()
}

/// Visitor that stops at the first path to one local binding.
struct BindingFinder<'cx, 'tcx> {
    /// Lint context used to resolve paths.
    cx: &'cx LateContext<'tcx>,
    /// Binding to find.
    hir_id: HirId,
}

impl<'tcx> Visitor<'tcx> for BindingFinder<'_, 'tcx> {
    type NestedFilter = OnlyBodies;
    type Result = ControlFlow<()>;

    /// Return the type context so closure bodies are visited.
    fn maybe_tcx(&mut self) -> TyCtxt<'tcx> {
        self.cx.tcx
    }

    /// Break at the first path that resolves to the binding.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) -> ControlFlow<()> {
        if is_binding_path(self.cx, expr, self.hir_id) {
            return ControlFlow::Break(());
        }
        walk_expr(self, expr)
    }
}

/// Emit the borrowed-input diagnostic.
fn emit_borrowed(cx: &LateContext<'_>, param: &SimpleParam) {
    let name = param.name;
    cx.emit_span_lint(
        OWNED_INPUT_FIELD_CLONES,
        param.span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "borrowed parameter `{name}` is cloned into the returned struct"
            ));
            let _ = diag.help("take ownership when the output must own data copied from the input");
        }),
    );
}

/// Emit the owned-input diagnostic.
///
/// The diagnostic carries a machine-applicable fix that moves the cloned
/// fields when the move provably compiles, and help text otherwise.
fn emit_owned(
    cx: &LateContext<'_>,
    param: &SimpleParam,
    clones: &[FieldClone<'_>],
    is_fix_applicable: bool,
) {
    let name = param.name;
    // Remove each `.clone()` suffix, from the end of the receiver to the end of the call.
    let removals: Vec<(Span, String)> = clones
        .iter()
        .map(|clone| {
            (
                clone.call.span.with_lo(clone.receiver.span.hi()),
                String::new(),
            )
        })
        .collect();
    let is_written_in_source = removals.iter().all(|(span, _)| !span.from_expansion());
    let can_apply_fix = is_fix_applicable && is_written_in_source;

    cx.emit_span_lint(
        OWNED_INPUT_FIELD_CLONES,
        param.span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(format!(
                "owned parameter `{name}` is only borrowed to clone fields into the returned struct"
            ));
            if can_apply_fix {
                let _ = diag.multipart_suggestion(
                    "move the fields out of the owned input",
                    removals,
                    Applicability::MachineApplicable,
                );
            } else {
                let _ = diag.help(
                    "consume the input by destructuring it, or take it by reference if cloning is intentional",
                );
            }
        }),
    );
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
