#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Detect constant Bevy `FontSize::Px` values at or outside the text-size bounds.
//! The check resolves the variant and evaluates supported f32 constants in
//! their owning bodies, including local constant paths and `+`, `-`, `*`, `/`.
//! It skips runtime values, trait-associated constants, unsupported casts, and
//! expressions deeper than sixteen levels instead of approximating Rust values.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;

use dylint_linting as _;
use rustc_hir::{
    BinOpKind, Expr, ExprKind,
    def::{CtorOf, DefKind, Res},
};
use rustc_lint::{LateContext, LateLintPass, LintContext as _};
use rustc_middle::ty;

#[cfg(test)]
use bevy as _;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub BEVY_INVALID_CONSTANT_FONT_SIZE,
    Warn,
    "a constant Bevy pixel font size is nonpositive or exceeds 1000 logical pixels",
    BevyInvalidConstantFontSize
}

impl<'tcx> LateLintPass<'tcx> for BevyInvalidConstantFontSize {
    /// Check resolved `FontSize::Px` constructors with statically evaluable values.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve only Bevy's enum variant before attempting constant evaluation.
        let Some(argument) = pixel_size_argument(cx, expr) else {
            return;
        };
        let Some(value) = constant_f32(cx, argument, 0) else {
            return;
        };

        // Apply Bevy's lower bound before its separate upper-size warning.
        if value <= 0.0 {
            cx.emit_span_lint(
                BEVY_INVALID_CONSTANT_FONT_SIZE,
                argument.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message("this constant pixel font size is nonpositive")
                        .help(
                            "use a positive size or disable the text through its visibility state",
                        );
                }),
            );
        } else if value > 1000.0 {
            cx.emit_span_lint(
                BEVY_INVALID_CONSTANT_FONT_SIZE,
                argument.span,
                rustc_errors::DiagDecorator(|diagnostic| {
                    let _configured_diagnostic = diagnostic
                        .primary_message(
                            "this constant pixel font size exceeds Bevy's 1000 logical pixel warning threshold",
                        )
                        .help("use a smaller raster size and scale the text entity when appropriate");
                }),
            );
        }
    }
}

/// Return the input to the resolved Bevy `FontSize::Px` tuple variant.
fn pixel_size_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // HIR represents this enum variant as a one-argument constructor call.
    let ExprKind::Call(callee, [argument]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(path) = callee.kind else {
        return None;
    };
    let Res::Def(DefKind::Ctor(CtorOf::Variant, _), constructor) =
        cx.typeck_results().qpath_res(&path, callee.hir_id)
    else {
        return None;
    };

    // The variant's parent is the enum, so aliases resolve without matching source spelling.
    let variant = cx.tcx.parent(constructor);
    let font_size = cx.tcx.parent(variant);
    (cx.tcx.crate_name(font_size.krate).as_str() == "bevy_text"
        && cx.tcx.item_name(font_size).as_str() == "FontSize"
        && cx.tcx.item_name(variant).as_str() == "Px")
        .then_some(argument)
}

/// Evaluate supported f32 literals, operations, and local constants.
fn constant_f32(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    // Bound recursion and reject values whose Rust expression type is not f32.
    if depth > 16 || !is_f32_expression(cx, expr) {
        return None;
    }

    // Each helper accepts one HIR form and preserves f32 rounding at every step.
    constant_float_literal(expr)
        .or_else(|| negated_constant(cx, expr, depth))
        .or_else(|| binary_constant(cx, expr, depth))
        .or_else(|| path_constant(cx, expr, depth))
        .or_else(|| block_constant(cx, expr, depth))
}

/// Parse a float literal using the f32 type resolved for its expression.
fn constant_float_literal(expr: &Expr<'_>) -> Option<f32> {
    let ExprKind::Lit(literal) = expr.kind else {
        return None;
    };
    let rustc_ast::LitKind::Float(value, _) = literal.node else {
        return None;
    };

    // Parsing directly as f32 applies Rust's literal rounding before arithmetic.
    value.as_str().replace('_', "").parse().ok()
}

/// Evaluate unary negation only when the operand also has a supported f32 value.
fn negated_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    let ExprKind::Unary(rustc_hir::UnOp::Neg, inner) = expr.kind else {
        return None;
    };

    // Negation preserves the input's floating-point representation.
    constant_f32(cx, inner, depth + 1).map(|value| -value)
}

/// Evaluate one supported binary operation at f32 precision.
fn binary_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    // Binary HIR nodes expose the resolved operator and its two operands.
    let ExprKind::Binary(operator, left, right) = expr.kind else {
        return None;
    };

    // Evaluate each operand recursively before applying its resolved operator.
    let left = constant_f32(cx, left, depth + 1)?;
    let right = constant_f32(cx, right, depth + 1)?;

    // Only operations listed in the lint contract enter the evaluator.
    let value = if operator.node == BinOpKind::Add {
        left + right
    } else if operator.node == BinOpKind::Sub {
        left - right
    } else if operator.node == BinOpKind::Mul {
        left * right
    } else if operator.node == BinOpKind::Div {
        left / right
    } else {
        return None;
    };
    Some(value)
}

/// Evaluate a local non-trait constant in its declaring body's type context.
fn path_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    // Resolve only paths whose definition is a constant item.
    let ExprKind::Path(path) = expr.kind else {
        return None;
    };
    let Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, definition) = cx
        .tcx
        .typeck(expr.hir_id.owner.def_id)
        .qpath_res(&path, expr.hir_id)
    else {
        return None;
    };

    // Trait declarations can be overridden, so their body does not prove a value.
    if cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait {
        return None;
    }

    // Read only a local initializer after excluding trait-level declarations.
    let body = definition
        .as_local()
        .and_then(|local| cx.tcx.hir_maybe_body_owned_by(local))?;
    constant_f32(cx, body.value, depth + 1)
}

/// Evaluate only an expression block with no statements.
fn block_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    let ExprKind::Block(block, _) = expr.kind else {
        return None;
    };
    if !block.stmts.is_empty() {
        return None;
    }

    // Statement-free blocks preserve the value and type of their tail expression.
    block
        .expr
        .and_then(|inner| constant_f32(cx, inner, depth + 1))
}

/// Resolve an expression's owning body.
///
/// Constant initializers need their own type-check results.
fn is_f32_expression(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    matches!(
        cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr).kind(),
        ty::Float(ty::FloatTy::F32)
    )
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
