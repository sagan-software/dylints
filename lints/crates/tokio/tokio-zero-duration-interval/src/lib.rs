#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for zero-duration Tokio intervals.
//!
//! The lint resolves calls to `tokio::time::interval` and `interval_at` and
//! reports a period that is a compile-time zero `Duration`, which makes Tokio
//! panic. A positive period keeps scheduling cooperative and makes the polling
//! cadence explicit for callers.

extern crate rustc_ast;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[cfg(test)]
use tokio as _;

use std::time::Duration as StandardDuration;

use rustc_hir::def::DefKind;
use rustc_hir::{BinOpKind, Expr, ExprKind, QPath, UnOp, def::Res};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{self, Ty};
use rustc_span::symbol::sym;
use tokio_support::{emit, is_zero_integer_constant, tokio_function_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub TOKIO_ZERO_DURATION_INTERVAL,
    Warn,
    "a Tokio interval is constructed with a zero period",
    TokioZeroDurationInterval
}

impl<'tcx> LateLintPass<'tcx> for TokioZeroDurationInterval {
    /// Check one call to a Tokio interval constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Normalize both interval constructors to their period argument.
        let period = tokio_function_call(cx, expr, "tokio::time::interval::interval")
            .and_then(|(_, arguments)| arguments.first())
            .or_else(|| {
                tokio_function_call(cx, expr, "tokio::time::interval::interval_at")
                    .and_then(|(_, arguments)| arguments.get(1))
            });
        if let Some(period) = period
            && is_zero_duration_period(cx, period)
        {
            emit(
                cx,
                TOKIO_ZERO_DURATION_INTERVAL,
                period.span,
                "Tokio interval period must be greater than zero",
                "use a positive duration",
                None,
            );
        }
    }
}

/// Evaluate a period only when its standard Duration value is statically known.
fn is_zero_duration_period<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    duration_value(cx, expr, 0).is_some_and(|duration| duration.is_zero())
}

/// Maximum expression and constant indirections evaluated for one duration.
const MAX_DURATION_EVALUATION_DEPTH: usize = 16;

/// Evaluate standard Duration expressions with known values.
fn duration_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Bound recursive expression and constant evaluation at one shared boundary.
    if depth >= MAX_DURATION_EVALUATION_DEPTH || !is_standard_duration_type(cx, expr) {
        return None;
    }

    duration_expression_value(cx, expr, depth)
}

/// Dispatch supported Duration expression forms to their evaluators.
fn duration_expression_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Keep each expression form behind its own typed evaluator.
    if let ExprKind::Path(ref path) = expr.kind {
        duration_path_value(cx, expr, path, depth)
    } else if let ExprKind::Call(callee, arguments) = expr.kind {
        duration_call_value(cx, callee, arguments, depth)
    } else if let ExprKind::Binary(operator, left, right) = expr.kind {
        duration_binary_value(cx, expr, operator.node, left, right, depth)
    } else if let ExprKind::Block(block, _) = expr.kind
        && block.stmts.is_empty()
    {
        // Statement-free blocks preserve their tail value without executing local work.
        block
            .expr
            .and_then(|tail| duration_value(cx, tail, depth + 1))
    } else {
        None
    }
}

/// Check whether an expression has the standard library's Duration type.
fn is_standard_duration_type<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>) -> bool {
    is_standard_duration_ty(cx, cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr))
}

/// Check whether a type is the standard library's re-exported core Duration.
fn is_standard_duration_ty<'tcx>(cx: &LateContext<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::TyKind::Adt(definition, _)
        if matches!(cx.tcx.def_path_str(definition.did()).as_str(),
            "core::time::Duration" | "std::time::Duration"))
}

/// Resolve a Duration constant or follow a local Duration constant's initializer.
fn duration_path_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    path: &QPath<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Resolve the path in its owning type-checking context.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let Res::Def(kind, definition) = typeck.qpath_res(path, expr.hir_id) else {
        return None;
    };
    let definition_path = cx.tcx.def_path_str(definition);

    // Recognize the standard constant before considering local initializers.
    if matches!(
        definition_path.as_str(),
        "core::time::Duration::ZERO" | "std::time::Duration::ZERO"
    ) {
        return Some(StandardDuration::ZERO);
    }

    // Only local non-trait constants have initializers that this lint can inspect.
    if !matches!(kind, DefKind::Const { .. } | DefKind::AssocConst { .. }) {
        return None;
    }
    let initializer = local_constant_initializer(cx, expr, path)?;
    duration_value(cx, initializer, depth + 1)
}

/// Evaluate a standard Duration constructor or its Default implementation.
fn duration_call_value<'tcx>(
    cx: &LateContext<'tcx>,
    callee: &Expr<'tcx>,
    arguments: &[Expr<'tcx>],
    depth: usize,
) -> Option<StandardDuration> {
    // Resolve only path calls because other call expressions have no supported constructor identity.
    let ExprKind::Path(path) = callee.kind else {
        return None;
    };
    let typeck = cx.tcx.typeck(callee.hir_id.owner.def_id);
    let Res::Def(_, definition) = typeck.qpath_res(&path, callee.hir_id) else {
        return None;
    };

    // Duration::default and Default::default both resolve to the standard trait method.
    if arguments.is_empty() && is_default_function(cx, definition) {
        return Some(StandardDuration::ZERO);
    }

    let constructor = standard_duration_constructor(cx, definition)?;
    duration_constructor_value(cx, constructor, arguments, depth)
}

/// Evaluate an already resolved constructor using the matching argument domain.
fn duration_constructor_value<'tcx>(
    cx: &LateContext<'tcx>,
    constructor: DurationConstructor,
    arguments: &[Expr<'tcx>],
    depth: usize,
) -> Option<StandardDuration> {
    // Keep integer and float constructor semantics separate.
    match constructor {
        DurationConstructor::Integer(constructor) => {
            duration_integer_constructor(cx, constructor, arguments, depth)
        }
        DurationConstructor::Float(constructor) => {
            duration_float_constructor(cx, constructor, arguments, depth)
        }
    }
}

/// The standard Duration constructors this evaluator can model exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DurationConstructor {
    /// An integer-backed constructor.
    Integer(IntegerDurationConstructor),
    /// A floating-point constructor.
    Float(FloatDurationConstructor),
}

/// Integer-backed standard Duration constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntegerDurationConstructor {
    /// Construct from whole seconds.
    FromSecs,
    /// Construct from milliseconds.
    FromMillis,
    /// Construct from microseconds.
    FromMicros,
    /// Construct from nanoseconds.
    FromNanos,
    /// Construct from seconds and nanoseconds.
    New,
}

/// Floating-point standard Duration constructors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FloatDurationConstructor {
    /// Construct from f32 seconds.
    F32,
    /// Construct from f64 seconds.
    F64,
}

/// Error returned when a resolved Duration member is not a supported constructor.
#[derive(Debug)]
struct UnknownDurationConstructor;

impl std::str::FromStr for DurationConstructor {
    type Err = UnknownDurationConstructor;

    /// Parse a defining member name into the supported constructor vocabulary.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "from_secs" => Ok(Self::Integer(IntegerDurationConstructor::FromSecs)),
            "from_millis" => Ok(Self::Integer(IntegerDurationConstructor::FromMillis)),
            "from_micros" => Ok(Self::Integer(IntegerDurationConstructor::FromMicros)),
            "from_nanos" => Ok(Self::Integer(IntegerDurationConstructor::FromNanos)),
            "new" => Ok(Self::Integer(IntegerDurationConstructor::New)),
            "from_secs_f32" => Ok(Self::Float(FloatDurationConstructor::F32)),
            "from_secs_f64" => Ok(Self::Float(FloatDurationConstructor::F64)),
            _ => Err(UnknownDurationConstructor),
        }
    }
}

/// Evaluate integer-based Duration constructors after name resolution.
fn duration_integer_constructor<'tcx>(
    cx: &LateContext<'tcx>,
    constructor: IntegerDurationConstructor,
    arguments: &[Expr<'tcx>],
    depth: usize,
) -> Option<StandardDuration> {
    // Dispatch only integer-backed constructor forms.
    match constructor {
        IntegerDurationConstructor::FromSecs => {
            duration_from_u64(cx, arguments, depth, StandardDuration::from_secs)
        }
        IntegerDurationConstructor::FromMillis => {
            duration_from_u64(cx, arguments, depth, StandardDuration::from_millis)
        }
        IntegerDurationConstructor::FromMicros => {
            duration_from_u64(cx, arguments, depth, StandardDuration::from_micros)
        }
        IntegerDurationConstructor::FromNanos => {
            duration_from_u64(cx, arguments, depth, StandardDuration::from_nanos)
        }
        IntegerDurationConstructor::New => duration_new(cx, arguments, depth),
    }
}

/// Apply a standard one-argument integer constructor to its known value.
fn duration_from_u64<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &[Expr<'tcx>],
    depth: usize,
    constructor: fn(u64) -> StandardDuration,
) -> Option<StandardDuration> {
    let [value] = arguments else { return None };
    Some(constructor(unsigned_u64(cx, value, depth + 1)?))
}

/// Normalize `Duration::new` arguments without overflowing its seconds field.
fn duration_new<'tcx>(
    cx: &LateContext<'tcx>,
    arguments: &[Expr<'tcx>],
    depth: usize,
) -> Option<StandardDuration> {
    // Require exactly two arguments before applying Duration's nanosecond normalization.
    let [seconds, nanoseconds] = arguments else {
        return None;
    };
    let seconds = unsigned_u64(cx, seconds, depth + 1)?;
    let nanoseconds = unsigned_u32(cx, nanoseconds, depth + 1)?;
    // Normalize excess nanoseconds into seconds using checked arithmetic.
    let carried_seconds = seconds.checked_add(u64::from(nanoseconds / 1_000_000_000))?;
    Some(StandardDuration::new(
        carried_seconds,
        nanoseconds % 1_000_000_000,
    ))
}

/// Evaluate exact float constructors after name resolution.
fn duration_float_constructor<'tcx>(
    cx: &LateContext<'tcx>,
    constructor: FloatDurationConstructor,
    arguments: &[Expr<'tcx>],
    depth: usize,
) -> Option<StandardDuration> {
    // Dispatch only constructors that preserve their declared floating-point width.
    match constructor {
        FloatDurationConstructor::F32 => {
            // Keep conversion in f32 to preserve its exact rounding behavior.
            let [seconds] = arguments else { return None };
            duration_from_float(cx, seconds, FloatDurationConstructor::F32, depth + 1)
        }
        FloatDurationConstructor::F64 => {
            // Keep conversion in f64 to preserve its exact rounding behavior.
            let [seconds] = arguments else { return None };
            duration_from_float(cx, seconds, FloatDurationConstructor::F64, depth + 1)
        }
    }
}

/// Parse a supported constructor from its standard-library defining path.
fn standard_duration_constructor(
    cx: &LateContext<'_>,
    definition: rustc_span::def_id::DefId,
) -> Option<DurationConstructor> {
    let path = cx.tcx.def_path_str(definition);
    let name = path
        .strip_prefix("core::time::Duration::")
        .or_else(|| path.strip_prefix("std::time::Duration::"))?;
    name.parse().ok()
}

/// Follow a local constant initializer when rustc resolves a path to one.
fn local_constant_initializer<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    path: &QPath<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    // Resolve the path in the expression owner's type-checking context.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, definition) =
        typeck.qpath_res(path, expr.hir_id)
    else {
        return None;
    };
    // Trait and external constants do not expose an inspectable local initializer.
    if cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait {
        return None;
    }
    let local = definition.as_local()?;
    cx.tcx.hir_maybe_body_owned_by(local).map(|body| body.value)
}

/// Check whether a definition is the standard `Default::default` method.
fn is_default_function(cx: &LateContext<'_>, definition: rustc_span::def_id::DefId) -> bool {
    cx.tcx.is_diagnostic_item(sym::default_fn, definition)
}

/// Evaluate standard Duration arithmetic with checked operations.
fn duration_binary_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    operator: BinOpKind,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Delegate each supported operator to the operand shape it requires.
    match operator {
        BinOpKind::Add => duration_pair_value(cx, DurationPairOperator::Add, left, right, depth),
        BinOpKind::Sub => duration_pair_value(cx, DurationPairOperator::Sub, left, right, depth),
        BinOpKind::Mul => duration_product_value(cx, expr, left, right, depth),
        BinOpKind::Div => duration_quotient_value(cx, expr, left, right, depth),
        BinOpKind::Rem
        | BinOpKind::And
        | BinOpKind::Or
        | BinOpKind::BitXor
        | BinOpKind::BitAnd
        | BinOpKind::BitOr
        | BinOpKind::Shl
        | BinOpKind::Shr
        | BinOpKind::Eq
        | BinOpKind::Lt
        | BinOpKind::Le
        | BinOpKind::Ne
        | BinOpKind::Ge
        | BinOpKind::Gt => None,
    }
}

/// Addition or subtraction applied to two Duration operands.
#[derive(Clone, Copy)]
enum DurationPairOperator {
    /// Add both known durations.
    Add,
    /// Subtract the right duration from the left duration.
    Sub,
}

/// Evaluate addition or subtraction between two known Duration operands.
fn duration_pair_value<'tcx>(
    cx: &LateContext<'tcx>,
    operator: DurationPairOperator,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Require two standard Duration operands before evaluating either side.
    if !is_standard_duration_type(cx, left) || !is_standard_duration_type(cx, right) {
        return None;
    }
    let left = duration_value(cx, left, depth + 1)?;
    let right = duration_value(cx, right, depth + 1)?;
    match operator {
        DurationPairOperator::Add => left.checked_add(right),
        DurationPairOperator::Sub => left.checked_sub(right),
    }
}

/// Evaluate multiplication when one operand is a Duration and the other is `u32`.
fn duration_product_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Require a Duration and u32 scalar before inspecting values.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let left_type = typeck.expr_ty(left);
    let right_type = typeck.expr_ty(right);

    // Accept either operand order while keeping the scalar type exact.
    match (
        is_standard_duration_ty(cx, left_type),
        is_standard_duration_ty(cx, right_type),
        is_u32(left_type),
        is_u32(right_type),
    ) {
        (true, false, _, true) => duration_scalar_product(cx, left, right, depth),
        (false, true, true, _) => duration_scalar_product(cx, right, left, depth),
        _ => None,
    }
}

/// Multiply a known Duration by a known `u32` scalar.
fn duration_scalar_product<'tcx>(
    cx: &LateContext<'tcx>,
    duration: &Expr<'tcx>,
    scalar: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    let duration = duration_value(cx, duration, depth + 1)?;
    let scalar = unsigned_u32(cx, scalar, depth + 1)?;
    duration.checked_mul(scalar)
}

/// Evaluate division when the left operand is Duration and the right is `u32`.
fn duration_quotient_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
) -> Option<StandardDuration> {
    // Read both operand types from the binary expression's owner.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    if !is_standard_duration_ty(cx, typeck.expr_ty(left)) || !is_u32(typeck.expr_ty(right)) {
        return None;
    }
    // Checked division returns unknown for a zero divisor or unrepresentable result.
    let duration = duration_value(cx, left, depth + 1)?;
    let divisor = unsigned_u32(cx, right, depth + 1)?;
    duration.checked_div(divisor)
}

/// Check whether a type is `u32`, the Duration scalar operator's integer type.
fn is_u32(ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::TyKind::Uint(ty::UintTy::U32))
}

/// Evaluate a supported unsigned integer expression at its declared width.
fn unsigned_integer_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
) -> Option<u128> {
    // Bound recursion before inspecting the expression's declared integer type.
    if depth >= MAX_DURATION_EVALUATION_DEPTH {
        return None;
    }
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let maximum = unsigned_integer_maximum(typeck.expr_ty(expr))?;

    unsigned_integer_expression_value(cx, expr, depth, maximum)
}

/// Dispatch the supported unsigned integer expression forms.
fn unsigned_integer_expression_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
    maximum: u128,
) -> Option<u128> {
    // Keep literal, path, arithmetic, and block evaluation separate.
    if let ExprKind::Lit(literal) = expr.kind {
        unsigned_integer_literal_value(cx, expr, literal, maximum)
    } else if let ExprKind::Path(ref path) = expr.kind {
        unsigned_integer_path_value(cx, expr, path, depth)
    } else if let ExprKind::Binary(operator, left, right) = expr.kind {
        unsigned_integer_binary_expression(cx, expr, operator.node, left, right, depth, maximum)
    } else if let ExprKind::Block(block, _) = expr.kind
        && block.stmts.is_empty()
    {
        // Ignore blocks with statements because evaluating them can have runtime effects.
        block
            .expr
            .and_then(|tail| unsigned_integer_value(cx, tail, depth + 1))
    } else {
        None
    }
}

/// Read an unsigned literal, including zero recognized by the shared Tokio helper.
fn unsigned_integer_literal_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    literal: rustc_hir::Lit,
    maximum: u128,
) -> Option<u128> {
    // Reuse the shared evaluator for an exact zero literal.
    if is_zero_integer_constant(cx, expr) {
        return Some(0);
    }
    let rustc_ast::LitKind::Int(value, _) = literal.node else {
        return None;
    };
    (value.get() <= maximum).then_some(value.get())
}

/// Follow a local unsigned integer constant initializer.
fn unsigned_integer_path_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    path: &QPath<'tcx>,
    depth: usize,
) -> Option<u128> {
    // The shared zero evaluator handles zero constants before nonzero recursion.
    if is_zero_integer_constant(cx, expr) {
        return Some(0);
    }
    let initializer = local_constant_initializer(cx, expr, path)?;
    unsigned_integer_value(cx, initializer, depth + 1)
}

/// Verify same-type operands before evaluating unsigned integer arithmetic.
fn unsigned_integer_binary_expression<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    operator: BinOpKind,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
    maximum: u128,
) -> Option<u128> {
    // Reject mixed-width or inferred-width arithmetic before applying checked operations.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let left_type = typeck.expr_ty(left);
    if left_type != typeck.expr_ty(right) || left_type != typeck.expr_ty(expr) {
        return None;
    }
    unsigned_binary_value(cx, operator, left, right, depth, maximum)
}

/// Return the upper bound for the integer widths used by Duration constructors.
fn unsigned_integer_maximum(ty: Ty<'_>) -> Option<u128> {
    // Only the unsigned widths accepted by standard Duration constructors are modeled.
    if matches!(ty.kind(), ty::TyKind::Uint(ty::UintTy::U32)) {
        return Some(u32::MAX.into());
    }
    if matches!(ty.kind(), ty::TyKind::Uint(ty::UintTy::U64)) {
        return Some(u64::MAX.into());
    }
    None
}

/// Evaluate same-type checked arithmetic for one unsigned integer expression.
fn unsigned_binary_value<'tcx>(
    cx: &LateContext<'tcx>,
    operator: BinOpKind,
    left: &Expr<'tcx>,
    right: &Expr<'tcx>,
    depth: usize,
    maximum: u128,
) -> Option<u128> {
    let left = unsigned_integer_value(cx, left, depth + 1)?;
    let right = unsigned_integer_value(cx, right, depth + 1)?;
    let value = match operator {
        BinOpKind::Add => left.checked_add(right),
        BinOpKind::Sub => left.checked_sub(right),
        BinOpKind::Mul => left.checked_mul(right),
        BinOpKind::Div => left.checked_div(right),
        BinOpKind::Rem => left.checked_rem(right),
        BinOpKind::And
        | BinOpKind::Or
        | BinOpKind::BitXor
        | BinOpKind::BitAnd
        | BinOpKind::BitOr
        | BinOpKind::Shl
        | BinOpKind::Shr
        | BinOpKind::Eq
        | BinOpKind::Lt
        | BinOpKind::Le
        | BinOpKind::Ne
        | BinOpKind::Ge
        | BinOpKind::Gt => None,
    }?;
    (value <= maximum).then_some(value)
}

/// Read an unsigned constructor argument as u64.
fn unsigned_u64<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, depth: usize) -> Option<u64> {
    u64::try_from(unsigned_integer_value(cx, expr, depth)?).ok()
}

/// Read an unsigned constructor argument as u32.
fn unsigned_u32<'tcx>(cx: &LateContext<'tcx>, expr: &Expr<'tcx>, depth: usize) -> Option<u32> {
    u32::try_from(unsigned_integer_value(cx, expr, depth)?).ok()
}

/// Preserve the source precision of one supported floating-point seconds value.
enum FloatSeconds {
    /// A value passed to `Duration::from_secs_f32`.
    F32(f32),
    /// A value passed to `Duration::from_secs_f64`.
    F64(f64),
}

/// Evaluate a float literal, local constant, negation, or statement-free block.
fn float_seconds_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    expected: FloatDurationConstructor,
    depth: usize,
) -> Option<FloatSeconds> {
    // Bound recursive evaluation and require the constructor's exact float width.
    if depth >= MAX_DURATION_EVALUATION_DEPTH || !is_expected_float_type(cx, expr, expected) {
        return None;
    }
    float_seconds_expression_value(cx, expr, depth, expected)
}

/// Check that an expression has the float width required by its constructor.
fn is_expected_float_type(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected: FloatDurationConstructor,
) -> bool {
    // Resolve the expression type in its owning HIR body.
    let typeck = cx.tcx.typeck(expr.hir_id.owner.def_id);
    let expression_type = typeck.expr_ty(expr);
    // Match only the width accepted by this constructor.
    match expected {
        FloatDurationConstructor::F32 => {
            matches!(expression_type.kind(), ty::TyKind::Float(ty::FloatTy::F32))
        }
        FloatDurationConstructor::F64 => {
            matches!(expression_type.kind(), ty::TyKind::Float(ty::FloatTy::F64))
        }
    }
}

/// Evaluate one supported float expression after resolving its exact float type.
fn float_seconds_expression_value<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    depth: usize,
    expected: FloatDurationConstructor,
) -> Option<FloatSeconds> {
    // Preserve source width while handling each supported expression form.
    if let ExprKind::Lit(literal) = expr.kind {
        float_seconds_literal(literal, expected)
    } else if let ExprKind::Path(ref path) = expr.kind {
        float_seconds_path(cx, expr, path, expected, depth)
    } else if let ExprKind::Unary(UnOp::Neg, operand) = expr.kind {
        float_seconds_negation(cx, operand, expected, depth)
    } else if let ExprKind::Block(block, _) = expr.kind
        && block.stmts.is_empty()
    {
        // Evaluate only an empty block's tail and preserve the recursion bound.
        block
            .expr
            .and_then(|tail| float_seconds_value(cx, tail, expected, depth + 1))
    } else {
        None
    }
}

/// Parse a floating-point literal without changing its source precision.
fn float_seconds_literal(
    literal: rustc_hir::Lit,
    expected: FloatDurationConstructor,
) -> Option<FloatSeconds> {
    // Parse the literal's exact spelling after removing only Rust separators.
    let rustc_ast::LitKind::Float(value, _) = literal.node else {
        return None;
    };
    let value = value.as_str().replace('_', "");
    match expected {
        FloatDurationConstructor::F32 => value.parse().ok().map(FloatSeconds::F32),
        FloatDurationConstructor::F64 => value.parse().ok().map(FloatSeconds::F64),
    }
}

/// Follow a local floating-point constant initializer.
fn float_seconds_path<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    path: &QPath<'tcx>,
    expected: FloatDurationConstructor,
    depth: usize,
) -> Option<FloatSeconds> {
    // Constant evaluation remains bounded by the shared recursion limit.
    let initializer = local_constant_initializer(cx, expr, path)?;
    float_seconds_value(cx, initializer, expected, depth + 1)
}

/// Apply unary negation without changing the floating-point width.
fn float_seconds_negation<'tcx>(
    cx: &LateContext<'tcx>,
    operand: &Expr<'tcx>,
    expected: FloatDurationConstructor,
    depth: usize,
) -> Option<FloatSeconds> {
    // Preserve signed zero and all source-width rounding behavior.
    match float_seconds_value(cx, operand, expected, depth + 1)? {
        FloatSeconds::F32(value) => Some(FloatSeconds::F32(-value)),
        FloatSeconds::F64(value) => Some(FloatSeconds::F64(-value)),
    }
}

/// Convert a known float through Duration's own exact nanosecond rounding.
fn duration_from_float<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &Expr<'tcx>,
    expected: FloatDurationConstructor,
    depth: usize,
) -> Option<StandardDuration> {
    // The evaluator's result carries the exact width used by Duration's conversion.
    match float_seconds_value(cx, expr, expected, depth)? {
        FloatSeconds::F32(seconds) => StandardDuration::try_from_secs_f32(seconds).ok(),
        FloatSeconds::F64(seconds) => StandardDuration::try_from_secs_f64(seconds).ok(),
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
