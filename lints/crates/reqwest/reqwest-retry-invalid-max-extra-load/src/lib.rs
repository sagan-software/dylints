#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "rustc syntax enums need a forward-compatible fallback"
)]

//! A lint to check for invalid Reqwest retry budget percentages.
//!
//! This Dylint library resolves Reqwest retry settings, reports invalid extra
//! load percentages, and recommends a value within the supported range.
//!
//! The README defines the finite boundary and diagnostic replacement. UI
//! fixtures cover valid, invalid, and non-literal inputs for safe adoption.

extern crate rustc_ast;
extern crate rustc_hir;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, reqwest_method_parts};
use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind, UnOp};
use rustc_lint::{LateContext, LateLintPass};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_RETRY_INVALID_MAX_EXTRA_LOAD,
    Warn,
    "a Reqwest retry budget percentage will panic",
    ReqwestRetryInvalidMaxExtraLoad
}

impl<'tcx> LateLintPass<'tcx> for ReqwestRetryInvalidMaxExtraLoad {
    /// Check literal retry-budget percentages against Reqwest's panic bounds.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact one-argument retry policy method before inspecting its value.
        let Some((_receiver, [extra_load], span)) =
            reqwest_method_parts(cx, expr, "max_extra_load")
        else {
            return;
        };
        // Reject only literals outside Reqwest's documented finite range.
        let Some(value) = numeric_literal(extra_load) else {
            return;
        };
        if value.is_valid_percentage() {
            return;
        }

        emit_span_lint_with_help(
            cx,
            REQWEST_RETRY_INVALID_MAX_EXTRA_LOAD,
            span,
            "Reqwest panics when `max_extra_load` is below 0.0 or above 1000.0",
            "use a finite value in the inclusive range 0.0 through 1000.0",
        );
    }
}

/// A numeric literal represented without a lossy integer-to-float conversion.
enum NumericLiteral {
    /// An integer literal with separately recorded sign.
    Integer {
        /// The unsigned magnitude from the parsed literal.
        magnitude: u128,
        /// Whether the expression applies unary negation.
        is_negative: bool,
    },
    /// A floating-point literal represented by its parsed value.
    Float(f64),
}

impl NumericLiteral {
    /// Return whether the literal satisfies Reqwest's inclusive percentage bounds.
    fn is_valid_percentage(&self) -> bool {
        match *self {
            Self::Integer {
                magnitude,
                is_negative,
            } => (!is_negative || magnitude == 0) && magnitude <= 1000,
            Self::Float(value) => (0.0..=1000.0).contains(&value),
        }
    }
}

/// Evaluate the literal numeric forms accepted by the retry builder.
fn numeric_literal(expr: &Expr<'_>) -> Option<NumericLiteral> {
    // Parse literal values before applying any unary sign.
    match expr.kind {
        ExprKind::Lit(literal) => literal_numeric(literal.node),
        // Apply one leading negation while rejecting double negation.
        ExprKind::Unary(UnOp::Neg, inner) => negative_numeric_literal(inner),
        _ => None,
    }
}

/// Parse an unsigned integer or floating-point literal.
fn literal_numeric(literal: LitKind) -> Option<NumericLiteral> {
    match literal {
        LitKind::Int(value, _) => Some(NumericLiteral::Integer {
            magnitude: value.get(),
            is_negative: false,
        }),
        LitKind::Float(value, _) => value
            .as_str()
            .replace('_', "")
            .parse()
            .ok()
            .map(NumericLiteral::Float),
        _ => None,
    }
}

/// Apply a single leading negative sign to one parsed literal.
fn negative_numeric_literal(expr: &Expr<'_>) -> Option<NumericLiteral> {
    match numeric_literal(expr)? {
        NumericLiteral::Integer {
            magnitude,
            is_negative: false,
        } => Some(NumericLiteral::Integer {
            magnitude,
            is_negative: true,
        }),
        NumericLiteral::Float(value) => Some(NumericLiteral::Float(-value)),
        NumericLiteral::Integer {
            is_negative: true, ..
        } => None,
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
