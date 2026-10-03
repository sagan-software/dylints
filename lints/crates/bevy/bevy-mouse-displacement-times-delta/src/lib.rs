#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detects multiplication of Bevy mouse displacement by elapsed time.
//! The pass resolves `MouseMotion` and `AccumulatedMouseMotion` fields and
//! non-fixed `Time` delta methods. It follows values through arithmetic, casts,
//! indexing, and branch results while skipping control tests and discarded
//! statements. It leaves velocity conversion, fixed-step time, keyboard input,
//! lookalike types, locals, helpers, and custom wrappers outside the warning
//! unless the exact Bevy sources remain visible.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;

use dylint_linting as _;
use rustc_hir::intravisit::Visitor;
use rustc_lint::LintContext as _;

#[cfg(test)]
use {bevy_ecs as _, bevy_input as _, bevy_time as _};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_MOUSE_DISPLACEMENT_TIMES_DELTA,
    Warn,
    "mouse displacement is multiplied by frame delta time",
    BevyMouseDisplacementTimesDelta
}

impl<'tcx> rustc_lint::LateLintPass<'tcx> for BevyMouseDisplacementTimesDelta {
    /// Find direct multiplication between resolved Bevy mouse displacement and time delta.
    fn check_expr(
        &mut self,
        cx: &rustc_lint::LateContext<'tcx>,
        expr: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        // Restrict diagnostics to multiplication nodes; operand walking handles nested arithmetic.
        let rustc_hir::ExprKind::Binary(operation, left, right) = expr.kind else {
            return;
        };
        if operation.node != rustc_hir::BinOpKind::Mul {
            return;
        }

        // Inspect each operand independently so mixed sources cannot hide in one side.
        let left_facts = expression_facts(cx, left);
        let right_facts = expression_facts(cx, right);
        let left_source = left_facts.source();
        let right_source = right_facts.source();
        let has_opposite_sources = matches!(
            (left_source, right_source),
            (ExpressionSource::MouseDelta, ExpressionSource::TimeDelta)
                | (ExpressionSource::TimeDelta, ExpressionSource::MouseDelta)
        );
        if !has_opposite_sources {
            return;
        }

        // Report the operator whose operands carry separate mouse and time values.
        cx.emit_span_lint(
            BEVY_MOUSE_DISPLACEMENT_TIMES_DELTA,
            expr.span,
            rustc_errors::DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("mouse displacement is multiplied by frame delta time")
                    .help("apply sensitivity to mouse displacement without multiplying by elapsed seconds");
            }),
        );
    }
}

/// Facts collected from one multiplication operand.
#[derive(Clone, Copy, Debug, Default)]
struct ExpressionFacts {
    /// Whether the operand reads Bevy mouse displacement.
    has_mouse_delta: bool,
    /// Whether the operand reads Bevy frame delta in seconds.
    has_time_delta: bool,
}

/// A source class collected from a value-producing expression.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExpressionSource {
    /// The expression has no recognized mouse or time source.
    None,
    /// The expression carries Bevy mouse displacement only.
    MouseDelta,
    /// The expression carries Bevy time delta only.
    TimeDelta,
    /// The expression combines mouse displacement and time delta.
    Mixed,
}

impl ExpressionFacts {
    /// Classify recognized sources without conflating mixed operand values.
    const fn source(self) -> ExpressionSource {
        match (self.has_mouse_delta, self.has_time_delta) {
            (false, false) => ExpressionSource::None,
            (true, false) => ExpressionSource::MouseDelta,
            (false, true) => ExpressionSource::TimeDelta,
            (true, true) => ExpressionSource::Mixed,
        }
    }
}

/// Collect sources that contribute to an expression's resulting value.
fn expression_facts<'tcx>(
    cx: &rustc_lint::LateContext<'tcx>,
    expr: &'tcx rustc_hir::Expr<'tcx>,
) -> ExpressionFacts {
    let mut visitor = ExpressionFactsVisitor {
        cx,
        facts: ExpressionFacts::default(),
    };
    visitor.visit_expr(expr);
    visitor.facts
}

/// Visitor that follows values while skipping control-flow tests and discarded statements.
struct ExpressionFactsVisitor<'cx, 'tcx> {
    /// Type context used to resolve Bevy fields and methods.
    cx: &'cx rustc_lint::LateContext<'tcx>,
    /// Resolved mouse and frame-time sources found in value positions.
    facts: ExpressionFacts,
}

impl<'tcx> Visitor<'tcx> for ExpressionFactsVisitor<'_, 'tcx> {
    /// Record direct sources and visit only expressions that contribute to the result value.
    fn visit_expr(&mut self, expr: &'tcx rustc_hir::Expr<'tcx>) {
        // Record only exact Bevy sources at the current expression.
        self.facts.has_mouse_delta |= is_mouse_delta_field(self.cx, expr);
        self.facts.has_time_delta |= is_time_delta_seconds(self.cx, expr);

        // Follow result-producing children and leave selectors and statements untouched.
        match expr.kind {
            rustc_hir::ExprKind::Binary(operation, left, right) => {
                self.visit_arithmetic_operands(operation.node, left, right);
            }
            rustc_hir::ExprKind::Unary(_, inner)
            | rustc_hir::ExprKind::Cast(inner, _)
            | rustc_hir::ExprKind::Type(inner, _)
            | rustc_hir::ExprKind::DropTemps(inner)
            | rustc_hir::ExprKind::AddrOf(_, _, inner) => self.visit_expr(inner),
            rustc_hir::ExprKind::Field(base, _) | rustc_hir::ExprKind::Index(base, _, _) => {
                self.visit_expr(base);
            }
            rustc_hir::ExprKind::Block(block, _) => self.visit_optional_value(block.expr),
            rustc_hir::ExprKind::If(_, then_value, else_value) => {
                self.visit_if_values(then_value, else_value);
            }
            rustc_hir::ExprKind::Match(_, arms, _) => self.visit_match_values(arms),
            // These forms do not propagate their child expressions into a value.
            rustc_hir::ExprKind::ConstBlock(_)
            | rustc_hir::ExprKind::Array(_)
            | rustc_hir::ExprKind::Call(..)
            | rustc_hir::ExprKind::MethodCall(..)
            | rustc_hir::ExprKind::Use(..)
            | rustc_hir::ExprKind::Tup(_)
            | rustc_hir::ExprKind::Lit(_)
            | rustc_hir::ExprKind::Let(_)
            | rustc_hir::ExprKind::Loop(..)
            | rustc_hir::ExprKind::Closure(_)
            | rustc_hir::ExprKind::Assign(..)
            | rustc_hir::ExprKind::AssignOp(..)
            | rustc_hir::ExprKind::Path(_)
            | rustc_hir::ExprKind::Break(..)
            | rustc_hir::ExprKind::Continue(_)
            | rustc_hir::ExprKind::Ret(_)
            | rustc_hir::ExprKind::Become(_)
            | rustc_hir::ExprKind::InlineAsm(_)
            | rustc_hir::ExprKind::OffsetOf(..)
            | rustc_hir::ExprKind::Struct(..)
            | rustc_hir::ExprKind::Repeat(..)
            | rustc_hir::ExprKind::Yield(..)
            | rustc_hir::ExprKind::UnsafeBinderCast(..)
            | rustc_hir::ExprKind::Err(_) => {}
        }
    }
}

impl<'tcx> ExpressionFactsVisitor<'_, 'tcx> {
    /// Visit both operands only when the binary operation carries their values.
    fn visit_arithmetic_operands(
        &mut self,
        operation: rustc_hir::BinOpKind,
        left: &'tcx rustc_hir::Expr<'tcx>,
        right: &'tcx rustc_hir::Expr<'tcx>,
    ) {
        // Comparisons and boolean operators select results instead of carrying both values.
        if is_value_arithmetic(operation) {
            self.visit_expr(left);
            self.visit_expr(right);
        }
    }

    /// Visit the result branches without treating the condition as a source value.
    fn visit_if_values(
        &mut self,
        then_value: &'tcx rustc_hir::Expr<'tcx>,
        else_value: Option<&'tcx rustc_hir::Expr<'tcx>>,
    ) {
        // Keep condition-only mouse reads outside the displacement value flow.
        self.visit_expr(then_value);
        self.visit_optional_value(else_value);
    }

    /// Visit an optional block or branch result when it contributes a value.
    fn visit_optional_value(&mut self, value: Option<&'tcx rustc_hir::Expr<'tcx>>) {
        if let Some(value) = value {
            self.visit_expr(value);
        }
    }

    /// Visit match results while skipping the scrutinee and arm guards.
    fn visit_match_values(&mut self, arms: &'tcx [rustc_hir::Arm<'tcx>]) {
        // Only arm bodies can contribute to the value returned by a match expression.
        for arm in arms {
            self.visit_expr(arm.body);
        }
    }
}

/// Identify arithmetic operators whose results depend on both numeric operands.
const fn is_value_arithmetic(operation: rustc_hir::BinOpKind) -> bool {
    matches!(
        operation,
        rustc_hir::BinOpKind::Add
            | rustc_hir::BinOpKind::Sub
            | rustc_hir::BinOpKind::Mul
            | rustc_hir::BinOpKind::Div
            | rustc_hir::BinOpKind::Rem
    )
}

/// Return whether a resolved field is `delta` on either Bevy mouse-motion type.
fn is_mouse_delta_field(cx: &rustc_lint::LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    // Require a real `delta` field before resolving the field base type.
    let rustc_hir::ExprKind::Field(base, field) = expr.kind else {
        return false;
    };
    if field.name.as_str() != "delta" {
        return false;
    }
    contains_mouse_motion_type(cx, cx.typeck_results().expr_ty_adjusted(base))
}

/// Return whether a field base is a mouse-motion type or an exact Bevy resource
/// wrapper around one.
fn contains_mouse_motion_type(
    cx: &rustc_lint::LateContext<'_>,
    ty: rustc_middle::ty::Ty<'_>,
) -> bool {
    // Match the concrete Bevy motion types before considering resource wrappers.
    let ty = ty.peel_refs();
    if bevy_support::type_is_named(cx, ty, "bevy_input", "MouseMotion")
        || bevy_support::type_is_named(cx, ty, "bevy_input", "AccumulatedMouseMotion")
    {
        return true;
    }
    // Limit wrapper traversal to Bevy's exact resource parameter types.
    let rustc_middle::ty::TyKind::Adt(wrapper, arguments) = ty.kind() else {
        return false;
    };
    let wrapper_id = wrapper.did();
    let is_resource = cx.tcx.crate_name(wrapper_id.krate).as_str() == "bevy_ecs"
        && matches!(cx.tcx.item_name(wrapper_id).as_str(), "Res" | "ResMut");
    is_resource
        && arguments.types().any(|argument| {
            bevy_support::type_is_named(cx, argument, "bevy_input", "MouseMotion")
                || bevy_support::type_is_named(cx, argument, "bevy_input", "AccumulatedMouseMotion")
        })
}

/// Return whether an expression resolves to time delta measured in seconds.
fn is_time_delta_seconds(cx: &rustc_lint::LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    // Prefer Bevy's direct seconds accessors when they resolve exactly.
    if is_time_method(cx, expr, "delta_secs") || is_time_method(cx, expr, "delta_secs_f64") {
        return true;
    }
    // Accept duration conversions only when the receiver is Bevy `Time::delta`.
    let rustc_hir::ExprKind::MethodCall(segment, receiver, _, _) = expr.kind else {
        return false;
    };
    if !matches!(segment.ident.name.as_str(), "as_secs_f32" | "as_secs_f64")
        || !bevy_support::type_is_named(
            cx,
            cx.typeck_results().expr_ty_adjusted(receiver),
            "core",
            "Duration",
        )
    {
        return false;
    }
    is_time_method(cx, receiver, "delta")
}

/// Return whether an expression resolves to one exact inherent method on `Time`.
fn is_time_method(
    cx: &rustc_lint::LateContext<'_>,
    expr: &rustc_hir::Expr<'_>,
    method_name: &str,
) -> bool {
    // Check the source-level method name before querying its resolved definition.
    let rustc_hir::ExprKind::MethodCall(segment, receiver, _, _) = expr.kind else {
        return false;
    };
    if segment.ident.name.as_str() != method_name {
        return false;
    }
    let Some(method) = cx.typeck_results().type_dependent_def_id(expr.hir_id) else {
        return false;
    };
    // Match the declaring impl so same-named application methods remain clean.
    let is_bevy_time_method = cx
        .tcx
        .inherent_impl_of_assoc(method)
        .is_some_and(|implementation| {
            bevy_support::type_is_named(
                cx,
                cx.tcx
                    .type_of(implementation)
                    .instantiate_identity()
                    .skip_norm_wip(),
                "bevy_time",
                "Time",
            )
        });
    is_bevy_time_method && !is_fixed_time_receiver(cx, receiver)
}

/// Return whether a resolved receiver is `Time<Fixed>` or its resource wrapper.
fn is_fixed_time_receiver(cx: &rustc_lint::LateContext<'_>, expr: &rustc_hir::Expr<'_>) -> bool {
    is_fixed_time_type(cx, cx.typeck_results().expr_ty_adjusted(expr))
}

/// Resolve `Time<Fixed>` through only Bevy's resource wrappers.
fn is_fixed_time_type(cx: &rustc_lint::LateContext<'_>, ty: rustc_middle::ty::Ty<'_>) -> bool {
    // Inspect only ADTs; fixed duration can be excluded only after type resolution.
    let rustc_middle::ty::TyKind::Adt(definition, arguments) = ty.peel_refs().kind() else {
        return false;
    };
    let def_id = definition.did();
    if cx.tcx.crate_name(def_id.krate).as_str() == "bevy_time"
        && cx.tcx.item_name(def_id).as_str() == "Time"
    {
        return arguments
            .types()
            .any(|argument| bevy_support::type_is_named(cx, argument, "bevy_time", "Fixed"));
    }

    // Preserve the fixed-time check when a resource wrapper remains visible.
    let is_resource_wrapper = cx.tcx.crate_name(def_id.krate).as_str() == "bevy_ecs"
        && matches!(cx.tcx.item_name(def_id).as_str(), "Res" | "ResMut");
    is_resource_wrapper
        && arguments
            .types()
            .any(|argument| is_fixed_time_type(cx, argument))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
