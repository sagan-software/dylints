#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! This lint checks directly registered Bevy systems in repeating schedules for
//! UI scales derived from recognized time and vector-distance values. It follows
//! those values through selected floating-point expressions and simple local
//! assignments. It reports potentially varying writes, but cannot prove runtime
//! variation, system reachability, text usage, or atlas creation.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

use std::collections::HashSet;

use dylint_linting as _;
use rustc_hir::{
    Body, Expr, ExprKind, FnDecl, HirId, LetStmt, PatKind,
    def::{CtorOf, DefKind, Res},
    def_id::LocalDefId,
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_middle::ty;
use rustc_span::{Span, Symbol};

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub BEVY_CONTINUOUS_UI_SCALE,
    Warn,
    "a system registered in a repeating schedule assigns Bevy UI scale from time or distance",
    BevyContinuousUiScale,
    BevyContinuousUiScale::default()
}

/// Candidate UI-scale write collected from one local function.
#[derive(Clone, Copy, Debug)]
struct Candidate {
    /// Function that contains the write.
    system: LocalDefId,
    /// Span of the changing time or distance source.
    source: Span,
}

/// Tracks candidate writes and potentially repeating registrations
/// until the crate pass ends.
///
/// The lint records writes while it visits function bodies, then reports only
/// writes whose function has a directly resolved registration in a repeating
/// Bevy schedule without a proven one-shot run condition. The visitor keeps
/// local data-flow state per function.
#[derive(Debug, Default)]
pub struct BevyContinuousUiScale {
    /// Candidate `UiScale` writes found in local functions.
    candidates: Vec<Candidate>,
    /// Free functions registered under conditions that may repeat.
    repeating_systems: Vec<LocalDefId>,
}

impl<'tcx> LateLintPass<'tcx> for BevyContinuousUiScale {
    /// Inspect local function bodies for writes to Bevy's `UiScale` resource.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        _: Span,
        local_def_id: LocalDefId,
    ) {
        let mut writes = UiScaleWriteVisitor {
            cx,
            system: local_def_id,
            continuous_locals: HashSet::new(),
            candidates: &mut self.candidates,
        };
        writes.visit_expr(body.value);
    }

    /// Retain direct registrations that may repeat under built-in schedules.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the registration before checking whether its schedule repeats.
        let Some(registration) = bevy_support::directly_registered_systems(cx, expr) else {
            return;
        };
        if !is_repeating_schedule(cx, expr, registration.schedule.as_str()) {
            return;
        }
        // Preserve condition semantics while retaining only repeatable functions.
        let Some(systems) = bevy_support::directly_registered_repeating_systems(cx, expr) else {
            return;
        };
        self.repeating_systems.extend(systems);
    }

    /// Report candidate writes only when their function is directly registered.
    fn check_crate_post(&mut self, cx: &LateContext<'tcx>) {
        // Suppress duplicate source spans from repeated HIR traversal paths.
        let mut emitted = HashSet::new();
        for candidate in &self.candidates {
            if !self.repeating_systems.contains(&candidate.system)
                || !emitted.insert((candidate.system, candidate.source))
            {
                continue;
            }
            cx.emit_span_lint(
                BEVY_CONTINUOUS_UI_SCALE,
                candidate.source,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "this system is registered in a repeating schedule and derives `UiScale` from a time- or distance-based value that may vary between runs",
                        )
                        .help(
                            "when the app renders text, prefer a stable or discrete scale because new raster sizes may use additional atlas keys",
                        );
                }),
            );
        }
    }
}

/// Collect UI-scale writes while tracking local values derived from time or distance.
struct UiScaleWriteVisitor<'a, 'tcx> {
    /// Type and method resolution for the current crate.
    cx: &'a LateContext<'tcx>,
    /// Function that contains each candidate write.
    system: LocalDefId,
    /// Locals whose current values derive from a supported continuous source.
    continuous_locals: HashSet<HirId>,
    /// Shared output list owned by the lint pass.
    candidates: &'a mut Vec<Candidate>,
}

impl<'tcx> Visitor<'tcx> for UiScaleWriteVisitor<'_, 'tcx> {
    /// Record direct resource writes and propagate derived local values.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        if matches!(expr.kind, ExprKind::Closure(..)) {
            return;
        }
        // Branch visitors merge local taint instead of walking those nodes twice.
        if let ExprKind::If(condition, then_expr, else_expr) = expr.kind {
            self.visit_if(condition, then_expr, else_expr);
            return;
        }
        if let ExprKind::Match(scrutinee, arms, _) = expr.kind {
            self.visit_match(scrutinee, arms);
            return;
        }

        // Record assignment state before the standard visitor explores operands.
        if let ExprKind::Assign(lhs, rhs, _) = expr.kind {
            self.update_continuous_local(lhs, rhs);
            self.collect_candidate(lhs, rhs);
        }
        if let ExprKind::AssignOp(operator, lhs, rhs) = expr.kind {
            self.update_compound_local(operator.node.into(), lhs, rhs);
            self.collect_candidate(lhs, rhs);
        }

        // Standard traversal visits nested expressions after recording the write.
        intravisit::walk_expr(self, expr);
    }

    /// Track initialized locals after visiting their initializer.
    fn visit_local(&mut self, local: &'tcx LetStmt<'tcx>) {
        if let Some(initializer) = local.init {
            self.visit_expr(initializer);
            if let PatKind::Binding(_, id, _, _) = local.pat.kind {
                // Bindings inherit taint only from supported changing sources.
                let is_continuous =
                    continuous_source(self.cx, initializer, &self.continuous_locals).is_some();
                if is_continuous {
                    self.continuous_locals.extend([id]);
                } else {
                    let _ = self.continuous_locals.remove(&id);
                }
            }
        }
        // Let-else bodies may contain independent writes even though they diverge.
        if let Some(else_block) = local.els {
            self.visit_block(else_block);
        }
    }

    /// Avoid attributing nested local function bodies to the enclosing system.
    fn visit_nested_item(&mut self, _: rustc_hir::ItemId) {}
}

impl<'tcx> UiScaleWriteVisitor<'_, 'tcx> {
    /// Merge the states of both `if` branches without evaluating the condition.
    fn visit_if(
        &mut self,
        condition: &'tcx Expr<'tcx>,
        then_expr: &'tcx Expr<'tcx>,
        else_expr: Option<&'tcx Expr<'tcx>>,
    ) {
        // Preserve writes in the condition before taking the branch snapshot.
        self.visit_expr(condition);
        let before = self.continuous_locals.clone();
        self.visit_expr(then_expr);
        let then_state = self.continuous_locals.clone();

        // Evaluate the else branch from the same incoming state as the then branch.
        self.continuous_locals.clone_from(&before);
        if let Some(else_expr) = else_expr {
            self.visit_expr(else_expr);
        }
        self.continuous_locals.extend(then_state);
    }

    /// Merge taint from each match arm while retaining scrutinee writes.
    fn visit_match(&mut self, scrutinee: &'tcx Expr<'tcx>, arms: &'tcx [rustc_hir::Arm<'tcx>]) {
        self.visit_expr(scrutinee);
        let before = self.continuous_locals.clone();
        let mut merged = HashSet::new();

        // Each arm starts from the state produced by the scrutinee.
        for arm in arms {
            self.continuous_locals.clone_from(&before);
            self.visit_expr(arm.body);
            merged.extend(self.continuous_locals.iter().copied());
        }
        // Keep taint from any arm that can write the value.
        self.continuous_locals = merged;
    }

    /// Update local taint when an assignment uses a supported source expression.
    fn update_continuous_local(&mut self, lhs: &'tcx Expr<'tcx>, rhs: &'tcx Expr<'tcx>) {
        let Some(id) = local_id(self.cx, lhs) else {
            return;
        };
        // A simple assignment overwrites earlier taint, including with a constant.
        if continuous_source(self.cx, rhs, &self.continuous_locals).is_some() {
            self.continuous_locals.extend([id]);
        } else {
            let _ = self.continuous_locals.remove(&id);
        }
    }

    /// Track compound arithmetic unless it provably overwrites taint with zero.
    fn update_compound_local(
        &mut self,
        operator: rustc_hir::BinOpKind,
        lhs: &'tcx Expr<'tcx>,
        rhs: &'tcx Expr<'tcx>,
    ) {
        let Some(id) = local_id(self.cx, lhs) else {
            return;
        };
        // Unsupported compound operators do not preserve the tracked value model.
        if !matches!(
            operator,
            rustc_hir::BinOpKind::Add
                | rustc_hir::BinOpKind::Sub
                | rustc_hir::BinOpKind::Mul
                | rustc_hir::BinOpKind::Div
        ) {
            let _ = self.continuous_locals.remove(&id);
            return;
        }

        // Multiplication by zero erases any continuous contribution.
        if operator == rustc_hir::BinOpKind::Mul
            && constant_number(self.cx, rhs, 0).is_some_and(ConstantNumber::is_zero)
        {
            let _ = self.continuous_locals.remove(&id);
            return;
        }

        // Preserve taint from either the previous value or the new right operand.
        let is_right_side_continuous =
            continuous_source(self.cx, rhs, &self.continuous_locals).is_some();
        if self.continuous_locals.contains(&id) || is_right_side_continuous {
            self.continuous_locals.extend([id]);
        }
    }

    /// Record a recognized `UiScale` write and its changing source span.
    fn collect_candidate(&mut self, lhs: &'tcx Expr<'tcx>, rhs: &'tcx Expr<'tcx>) {
        // Resolve the resource field or whole-resource constructor first.
        if let Some(value) = assigned_ui_scale(self.cx, lhs, rhs)
            && let Some(source) = continuous_source(self.cx, value, &self.continuous_locals)
        {
            self.candidates.push(Candidate {
                system: self.system,
                source,
            });
        }
    }
}

/// Return the assigned value when the left side is the resolved Bevy `UiScale` value.
fn assigned_ui_scale<'hir>(
    cx: &LateContext<'_>,
    lhs: &'hir Expr<'hir>,
    rhs: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // Resource-field assignments expose the scalar directly.
    if let ExprKind::Field(base, field) = lhs.kind
        && field.name.as_str() == "0"
        && bevy_support::expression_has_type(cx, base, "bevy_ui", "UiScale")
    {
        return Some(rhs);
    }

    // Whole-resource writes require a resolved UiScale value on both sides.
    if !bevy_support::expression_has_type(cx, lhs, "bevy_ui", "UiScale")
        || !bevy_support::expression_has_type(cx, rhs, "bevy_ui", "UiScale")
    {
        return None;
    }

    ui_scale_constructor_argument(cx, rhs)
}

/// Return the argument to Bevy's tuple-struct constructor for `UiScale`.
fn ui_scale_constructor_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // HIR represents tuple-struct constructors as one-argument calls.
    let ExprKind::Call(callee, [argument]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(path) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Ctor(CtorOf::Struct, _), constructor) =
        cx.typeck_results().qpath_res(&path, callee.hir_id)
    else {
        return None;
    };

    // Resolve the constructor's parent type instead of matching an alias spelling.
    let struct_id = cx.tcx.parent(constructor);
    (cx.tcx.crate_name(struct_id.krate).as_str() == "bevy_ui"
        && cx.tcx.item_name(struct_id).as_str() == "UiScale")
        .then_some(argument)
}

/// Return the local binding referenced by a resolved path or direct dereference.
fn local_id(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<HirId> {
    // Paths retain data-flow identity after name resolution.
    if let ExprKind::Path(path) = expr.kind {
        return if let Res::Local(id) = cx.qpath_res(&path, expr.hir_id) {
            Some(id)
        } else {
            None
        };
    }

    // Dereferencing a local resource keeps the underlying binding identity.
    if let ExprKind::Unary(rustc_hir::UnOp::Deref, inner) = expr.kind {
        return local_id(cx, inner);
    }

    None
}

/// Return a resolved time or vector-distance source that reaches this value.
fn continuous_source<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    continuous_locals: &HashSet<HirId>,
) -> Option<Span> {
    // Resolve local and branch values before arithmetic or method sources.
    source_from_branches(cx, expr, continuous_locals)
        .or_else(|| source_from_expression(cx, expr, continuous_locals))
        .or_else(|| time_source(cx, expr))
        .or_else(|| is_vec3_distance(cx, expr).then_some(expr.span))
}

/// Trace resolved local references and values selected by branches.
fn source_from_branches<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    continuous_locals: &HashSet<HirId>,
) -> Option<Span> {
    if let ExprKind::Path(path) = expr.kind {
        // Resolved local identities preserve aliases tracked by the visitor.
        return if let Res::Local(id) = cx.qpath_res(&path, expr.hir_id)
            && continuous_locals.contains(&id)
        {
            Some(expr.span)
        } else {
            None
        };
    }

    if let ExprKind::If(_, then_expr, else_expr) = expr.kind {
        // Both value branches count; the condition is not evaluated.
        return continuous_source(cx, then_expr, continuous_locals).or_else(|| {
            else_expr.and_then(|value| continuous_source(cx, value, continuous_locals))
        });
    }

    if let ExprKind::Match(_, arms, _) = expr.kind {
        // Any arm can contribute a source, regardless of its guard.
        return arms
            .iter()
            .find_map(|arm| continuous_source(cx, arm.body, continuous_locals));
    }

    None
}

/// Trace supported arithmetic and methods through their input expressions.
fn source_from_expression<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    continuous_locals: &HashSet<HirId>,
) -> Option<Span> {
    if let ExprKind::Unary(rustc_hir::UnOp::Neg, value) | ExprKind::Cast(value, _) = expr.kind
        && is_float_type(cx.typeck_results().expr_ty(expr))
    {
        // Unary negation and supported casts retain the changing input.
        return continuous_source(cx, value, continuous_locals);
    }

    if let ExprKind::Binary(operator, left, right) = expr.kind {
        // Arithmetic accepts one recognized source and a constant peer.
        return binary_source(cx, expr, operator.node, left, right, continuous_locals);
    }

    if let ExprKind::MethodCall(segment, receiver, _, _) = expr.kind {
        // Quantizers produce discrete values and are excluded from this rule.
        if is_float_quantizer(cx, expr, receiver, segment.ident.name) {
            return None;
        }
        if is_float_type(cx.typeck_results().expr_ty(expr)) && is_continuous_float_method(cx, expr)
        {
            return continuous_source(cx, receiver, continuous_locals);
        }
    }

    None
}

/// Trace supported floating-point arithmetic with one changing input and one constant.
fn binary_source<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
    operator: rustc_hir::BinOpKind,
    left: &'tcx Expr<'tcx>,
    right: &'tcx Expr<'tcx>,
    continuous_locals: &HashSet<HirId>,
) -> Option<Span> {
    // Reject non-floating expressions before following either operand.
    if !is_float_type(cx.typeck_results().expr_ty(expr)) {
        return None;
    }
    let left_source = continuous_source(cx, left, continuous_locals);
    let right_source = continuous_source(cx, right, continuous_locals);

    // Constants distinguish one changing operand from two changing values.
    let left_constant = constant_number(cx, left, 0);
    let right_constant = constant_number(cx, right, 0);
    if operator == rustc_hir::BinOpKind::Add || operator == rustc_hir::BinOpKind::Sub {
        additive_source(left_source, right_source, left_constant, right_constant)
    } else if operator == rustc_hir::BinOpKind::Mul || operator == rustc_hir::BinOpKind::Div {
        scaled_source(left_source, right_source, left_constant, right_constant)
    } else {
        None
    }
}

/// Keep an additive expression when exactly one operand is a changing source.
const fn additive_source(
    left_source: Option<Span>,
    right_source: Option<Span>,
    left_constant: Option<ConstantNumber>,
    right_constant: Option<ConstantNumber>,
) -> Option<Span> {
    match (left_source, right_source, left_constant, right_constant) {
        (Some(source), None, _, Some(_)) | (None, Some(source), Some(_), _) => Some(source),
        _ => None,
    }
}

/// Keep multiplication and division only when the constant factor is nonzero.
const fn scaled_source(
    left_source: Option<Span>,
    right_source: Option<Span>,
    left_constant: Option<ConstantNumber>,
    right_constant: Option<ConstantNumber>,
) -> Option<Span> {
    match (left_source, right_source, left_constant, right_constant) {
        (Some(source), None, _, Some(constant)) if !constant.is_zero() => Some(source),
        (None, Some(source), Some(constant), _) if !constant.is_zero() => Some(source),
        _ => None,
    }
}

/// Numeric constant retained at the precision declared by Rust's type checker.
#[derive(Clone, Copy, Debug)]
enum ConstantNumber {
    /// A single-precision constant.
    F32(f32),
    /// A double-precision constant.
    F64(f64),
}

impl ConstantNumber {
    /// Return whether this floating-point value equals zero, including negative zero.
    const fn is_zero(self) -> bool {
        match self {
            Self::F32(value) => value == 0.0,
            Self::F64(value) => value == 0.0,
        }
    }
}

/// Floating-point width resolved for one HIR expression.
#[derive(Clone, Copy, Debug)]
enum FloatWidth {
    /// Single precision.
    F32,
    /// Double precision.
    F64,
}

/// Resolve the expression's floating-point type from its owning body.
fn float_width(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<FloatWidth> {
    // Constant initializer nodes need their own owner-specific type context.
    let kind = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr).kind();
    if matches!(kind, ty::Float(ty::FloatTy::F32)) {
        return Some(FloatWidth::F32);
    }
    if matches!(kind, ty::Float(ty::FloatTy::F64)) {
        return Some(FloatWidth::F64);
    }

    // Values outside f32 and f64 are not supported constants.
    None
}

/// Evaluate supported floating-point constants without changing Rust's rounding precision.
fn constant_number(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<ConstantNumber> {
    // Bound recursion through both syntax trees and nested const paths.
    if depth > 16 {
        return None;
    }

    // Keep evaluation type-aware by delegating each supported HIR form.
    (if let ExprKind::Lit(literal) = expr.kind {
        constant_float_literal(cx, expr, literal)
    } else {
        None
    })
    .or_else(|| {
        if let ExprKind::Unary(rustc_hir::UnOp::Neg, inner) = expr.kind {
            negated_constant(cx, expr, inner, depth)
        } else {
            None
        }
    })
    .or_else(|| {
        if let ExprKind::Binary(operator, left, right) = expr.kind {
            binary_constant(cx, expr, operator.node, left, right, depth)
        } else {
            None
        }
    })
    .or_else(|| {
        if let ExprKind::Path(path) = expr.kind {
            path_constant(cx, expr, path, depth)
        } else {
            None
        }
    })
}

/// Parse one float literal at its type-checked precision.
fn constant_float_literal(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    literal: rustc_hir::Lit,
) -> Option<ConstantNumber> {
    // Only Rust floating-point literal tokens enter the numeric evaluator.
    let rustc_ast::LitKind::Float(value, _) = literal.node else {
        return None;
    };
    let value = value.as_str().replace('_', "");

    // Parse at the resolved width to preserve each operation's Rust rounding.
    match float_width(cx, expr)? {
        FloatWidth::F32 => value.parse().ok().map(ConstantNumber::F32),
        FloatWidth::F64 => value.parse().ok().map(ConstantNumber::F64),
    }
}

/// Negate a constant while keeping the expression's resolved width.
fn negated_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    inner: &Expr<'_>,
    depth: usize,
) -> Option<ConstantNumber> {
    // Both the expression and operand must resolve to the same float width.
    match (
        float_width(cx, expr)?,
        constant_number(cx, inner, depth + 1)?,
    ) {
        (FloatWidth::F32, ConstantNumber::F32(value)) => Some(ConstantNumber::F32(-value)),
        (FloatWidth::F64, ConstantNumber::F64(value)) => Some(ConstantNumber::F64(-value)),
        _ => None,
    }
}

/// Evaluate supported arithmetic with the width resolved for its owning body.
fn binary_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    operator: rustc_hir::BinOpKind,
    left: &Expr<'_>,
    right: &Expr<'_>,
    depth: usize,
) -> Option<ConstantNumber> {
    // Resolve the result width before evaluating both operands.
    let width = float_width(cx, expr)?;
    let left = constant_number(cx, left, depth + 1)?;
    let right = constant_number(cx, right, depth + 1)?;

    // Reject casts or mixed types instead of approximating their conversion.
    match (width, left, right) {
        (FloatWidth::F32, ConstantNumber::F32(left), ConstantNumber::F32(right)) => {
            binary_f32(operator, left, right)
        }
        (FloatWidth::F64, ConstantNumber::F64(left), ConstantNumber::F64(right)) => {
            binary_f64(operator, left, right)
        }
        _ => None,
    }
}

/// Apply one supported operator using single-precision arithmetic.
fn binary_f32(operator: rustc_hir::BinOpKind, left: f32, right: f32) -> Option<ConstantNumber> {
    // Perform each operation at f32 precision before its result is inspected.
    let value = if operator == rustc_hir::BinOpKind::Add {
        left + right
    } else if operator == rustc_hir::BinOpKind::Sub {
        left - right
    } else if operator == rustc_hir::BinOpKind::Mul {
        left * right
    } else if operator == rustc_hir::BinOpKind::Div {
        left / right
    } else {
        return None;
    };

    // The selected branch leaves a value only for supported arithmetic.
    Some(ConstantNumber::F32(value))
}

/// Apply one supported operator using double-precision arithmetic.
fn binary_f64(operator: rustc_hir::BinOpKind, left: f64, right: f64) -> Option<ConstantNumber> {
    // Preserve f64 arithmetic for literals and constant initializers.
    let value = if operator == rustc_hir::BinOpKind::Add {
        left + right
    } else if operator == rustc_hir::BinOpKind::Sub {
        left - right
    } else if operator == rustc_hir::BinOpKind::Mul {
        left * right
    } else if operator == rustc_hir::BinOpKind::Div {
        left / right
    } else {
        return None;
    };

    // The selected branch leaves a value only for supported arithmetic.
    Some(ConstantNumber::F64(value))
}

/// Resolve a local constant path and evaluate its initializer in that body's type context.
fn path_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    path: rustc_hir::QPath<'_>,
    depth: usize,
) -> Option<ConstantNumber> {
    // Resolve only local constants and inherent associated constants.
    let Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, definition) =
        cx.qpath_res(&path, expr.hir_id)
    else {
        return None;
    };
    if cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait {
        return None;
    }
    let local = definition.as_local()?;
    let initializer = cx.tcx.hir_maybe_body_owned_by(local)?.value;

    // The initializer's owner supplies its actual floating-point type.
    constant_number(cx, initializer, depth + 1)
}
/// Allow only core floating-point methods whose scalar result varies continuously.
fn is_continuous_float_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve method identity before comparing names from the core allowlist.
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    let method_name = cx.tcx.item_name(method);
    let name = method_name.as_str();
    cx.tcx.crate_name(method.krate).as_str() == "core"
        && matches!(
            name,
            "abs"
                | "acos"
                | "asin"
                | "atan"
                | "atan2"
                | "cbrt"
                | "cos"
                | "cosh"
                | "exp"
                | "exp2"
                | "hypot"
                | "ln"
                | "log"
                | "log10"
                | "log2"
                | "powf"
                | "powi"
                | "recip"
                | "sin"
                | "sinh"
                | "sqrt"
                | "tan"
                | "tanh"
                | "to_degrees"
                | "to_radians"
                | "as_secs_f32"
                | "as_secs_f64"
        )
}

/// Resolve one supported elapsed-time or frame-delta method on Bevy's `Time` type.
fn time_source(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Match supported methods on Bevy's resolved Time type.
    let call = [
        "elapsed_secs",
        "elapsed_secs_f64",
        "elapsed_secs_wrapped",
        "elapsed_secs_wrapped_f64",
        "delta_secs",
        "delta_secs_f64",
        "elapsed",
        "delta",
    ]
    .into_iter()
    .find_map(|name| bevy_support::bevy_method_call(cx, expr, "bevy_time", "Time", name))?;
    let is_delta = matches!(
        cx.tcx.item_name(call.def_id).as_str(),
        "delta" | "delta_secs" | "delta_secs_f64"
    );
    if is_delta && is_fixed_time(cx, expr, call.receiver) {
        None
    } else {
        Some(call.method_span)
    }
}

/// Detect `Time<Fixed>` so its stable timestep delta is not treated as changing.
fn is_fixed_time(cx: &LateContext<'_>, method_call: &Expr<'_>, receiver: &Expr<'_>) -> bool {
    // Inspect the receiver and method substitutions for the Fixed marker.
    let receiver_type = cx.typeck_results().expr_ty_adjusted(receiver).peel_refs();
    let receiver_is_fixed_time = if let ty::Adt(definition, arguments) = receiver_type.kind() {
        cx.tcx.crate_name(definition.did().krate).as_str() == "bevy_time"
            && cx.tcx.item_name(definition.did()).as_str() == "Time"
            && arguments.types().next().is_some_and(|argument| {
                bevy_support::type_is_named(cx, argument, "bevy_time", "Fixed")
            })
    } else {
        false
    };
    receiver_is_fixed_time
        || cx
            .typeck_results()
            .node_args(method_call.hir_id)
            .types()
            .any(|argument| bevy_support::type_is_named(cx, argument, "bevy_time", "Fixed"))
}

/// Match `Vec3::distance` through the resolved `glam::Vec3` method implementation.
fn is_vec3_distance(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Require one argument and the exact resolved glam method identity.
    let ExprKind::MethodCall(_, receiver, arguments, _) = expr.kind else {
        return false;
    };
    let [argument] = arguments else {
        return false;
    };
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };

    // Constant path pairs are fixed for the current monomorphization.
    cx.tcx.crate_name(method.krate).as_str() == "glam"
        && cx.tcx.item_name(method).as_str() == "distance"
        && bevy_support::expression_has_type(cx, receiver, "glam", "Vec3")
        && !(is_constant_vec3_operand(cx, receiver) && is_constant_vec3_operand(cx, argument))
}

/// Check whether a vector operand is a resolved constant path.
fn is_constant_vec3_operand(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::Path(path) = expr.kind else {
        return false;
    };

    // Resolved constant paths cannot change during a repeated system run.
    matches!(
        cx.typeck_results().qpath_res(&path, expr.hir_id),
        Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, _)
    )
}

/// Treat only the floating-point rounding functions as raster-size quantizers.
fn is_float_quantizer(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    receiver: &Expr<'_>,
    name: Symbol,
) -> bool {
    // Rounding methods can make a changing float source discrete.
    let is_rounding_method = matches!(
        name.as_str(),
        "floor" | "ceil" | "round" | "trunc" | "round_ties_even"
    );
    is_rounding_method
        && matches!(
            cx.typeck_results().expr_ty_adjusted(receiver).kind(),
            ty::Float(_)
        )
        && cx
            .typeck_results()
            .type_dependent_def_id(expr.hir_id)
            .is_some_and(|method| {
                let crate_name = cx.tcx.crate_name(method.krate);
                crate_name.as_str() == "core" || crate_name.as_str() == "std"
            })
}

/// Return whether the type is a floating-point primitive.
fn is_float_type(ty: ty::Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Float(_))
}

/// Keep detection to the standard schedules that may run repeatedly after startup.
fn is_repeating_schedule(cx: &LateContext<'_>, expr: &Expr<'_>, schedule: &str) -> bool {
    // Restrict candidates to built-in schedules that can repeat after startup.
    if !matches!(
        schedule,
        "First"
            | "PreUpdate"
            | "Update"
            | "PostUpdate"
            | "Last"
            | "FixedFirst"
            | "FixedPreUpdate"
            | "FixedUpdate"
            | "FixedPostUpdate"
            | "FixedLast"
    ) {
        return false;
    }

    // Resolve the registered method only after the schedule name is recognized.
    let Some(call) = bevy_support::bevy_method_call(cx, expr, "bevy_app", "App", "add_systems")
        .or_else(|| bevy_support::bevy_method_call(cx, expr, "bevy_app", "SubApp", "add_systems"))
    else {
        return false;
    };
    let Some(schedule_expression) = call.arguments.first() else {
        return false;
    };
    bevy_support::expression_has_type(cx, schedule_expression, "bevy_app", schedule)
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
