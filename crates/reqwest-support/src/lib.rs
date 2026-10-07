#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Reqwest-specific lints.
//!
//! The helpers resolve Reqwest methods and functions through rustc metadata,
//! preserve diagnostic spans, and reject local lookalikes with identical names.
//! They keep network-client lint rules small, deterministic, and independent of
//! source-text heuristics that cannot prove the defining crate.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    BinOpKind, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, Node, UnOp,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, Lint, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, Symbol, def_id::DefId};

use dylint_linting as _;

/// Maximum number of boolean expression nodes inspected for one Reqwest argument.
const MAX_BOOLEAN_EXPR_NODES: usize = 16;

/// One semantically resolved Reqwest method call with its canonical definition.
///
/// The result carries the source span, method name, and resolved path needed by
/// diagnostics that distinguish Reqwest APIs from local extension methods.
#[derive(Clone, Debug)]
pub struct ReqwestMethod {
    /// User-facing method-name span used for the primary diagnostic.
    pub span: Span,
    /// Resolved method name obtained from type-dependent method lookup.
    pub name: Symbol,
    /// Canonical Rust definition path used to identify the external API.
    pub definition: String,
}

/// One semantically resolved Reqwest associated-function or free-function call.
///
/// This value keeps the original callee span and resolved metadata together so
/// callers do not repeat path resolution or accidentally accept local functions.
#[derive(Clone, Debug)]
pub struct ReqwestFunction {
    /// User-facing callee span used for the primary diagnostic.
    pub span: Span,
    /// Resolved item name obtained from rustc's definition metadata.
    pub name: Symbol,
    /// Canonical Rust definition path used to identify the external API.
    pub definition: String,
}

/// Resolve a method call only when the method is defined by Reqwest.
///
/// Type-dependent resolution excludes extension traits and local methods that
/// merely reuse a Reqwest method spelling in application code.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::reqwest_method(cx, expr);
/// };
/// ```
#[must_use]
pub fn reqwest_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<ReqwestMethod> {
    // Type-dependent resolution rejects extension traits and user methods with the same spelling.
    let ExprKind::MethodCall(segment, ..) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_reqwest_def(cx, def_id).then_some(ReqwestMethod {
        span: segment.ident.span,
        name: segment.ident.name,
        definition: cx.tcx.def_path_str(def_id),
    })
}

/// Resolve a direct call only when its callee is defined by Reqwest.
///
/// The helper accepts only a path callee and a resolved Reqwest definition, so
/// closures and same-named local functions remain outside the lint contract.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::reqwest_function(cx, expr);
/// };
/// ```
#[must_use]
pub fn reqwest_function(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<ReqwestFunction> {
    // Restrict semantic resolution to direct calls with path callees.
    let ExprKind::Call(callee, _) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };

    // Reject same-named functions unless Reqwest owns the resolved definition.
    is_reqwest_def(cx, def_id).then_some(ReqwestFunction {
        span: callee.span,
        name: cx.tcx.item_name(def_id),
        definition: cx.tcx.def_path_str(def_id),
    })
}

/// Return the receiver and arguments for a resolved Reqwest method.
///
/// The expected method name is checked after semantic resolution, preserving the
/// receiver and argument slices needed by the caller without cloning HIR nodes.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_name| {
///     let _ = reqwest_support::reqwest_method_parts(cx, expr, expected_name);
/// };
/// ```
#[must_use]
pub fn reqwest_method_parts<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_name: &str,
) -> Option<(&'hir Expr<'hir>, &'hir [Expr<'hir>], Span)> {
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    let method = reqwest_method(cx, expr)?;
    (method.name.as_str() == expected_name).then_some((receiver, arguments, segment.ident.span))
}

/// Return whether an expression is lexically nested in a loop.
///
/// Parent traversal identifies an enclosing HIR loop even when compiler wrappers
/// occur between the expression and its source-level loop body.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::is_in_loop(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    cx.tcx.hir_parent_iter(expr.hir_id).any(
        |(_, node)| matches!(node, Node::Expr(parent) if matches!(parent.kind, ExprKind::Loop(..))),
    )
}

/// Return whether the expression runs in the nearest async body.
///
/// The nearest closure boundary determines the result, preventing synchronous
/// nested closures from inheriting asynchronous state from their parent function.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::is_in_async_body(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_in_async_body(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Stop at the nearest closure so a spawn-blocking closure stays synchronous.
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        let Node::Expr(parent) = node else {
            continue;
        };
        let ExprKind::Closure(closure) = parent.kind else {
            continue;
        };

        // The nearest closure determines whether this expression executes asynchronously.
        return matches!(
            closure.kind,
            ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
                | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
        );
    }

    false
}

/// Return whether an expression is the literal `true`.
///
/// Only the exact boolean literal matches; names, constant expressions, and
/// converted values are intentionally outside this source-level helper's scope.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |expr| {
///     let _ = reqwest_support::is_true_literal(expr);
/// };
/// ```
#[must_use]
pub const fn is_true_literal(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal) if matches!(literal.node, LitKind::Bool(true)))
}

/// Return whether a bounded boolean expression is known to evaluate to `true`.
///
/// The analysis accepts boolean literals, local boolean `const` items, `!`,
/// `&&`, and `||`. It follows short-circuit order and leaves runtime values,
/// associated constants, external constants, and over-budget expressions unknown.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::is_true_boolean_expression(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_true_boolean_expression(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let mut remaining_nodes = MAX_BOOLEAN_EXPR_NODES;
    known_boolean_value(cx, expr, &mut remaining_nodes) == Some(true)
}

/// Resolve a boolean literal, local constant, or supported boolean operation.
fn known_boolean_value(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    remaining_nodes: &mut usize,
) -> Option<bool> {
    // Stop before traversing an expression whose analysis would exceed the fixed node budget.
    if *remaining_nodes == 0 {
        return None;
    }
    *remaining_nodes -= 1;

    if let ExprKind::Lit(literal) = expr.kind {
        if let LitKind::Bool(value) = literal.node {
            return Some(value);
        }
        return None;
    }
    if let ExprKind::Path(path) = expr.kind {
        return local_boolean_constant(cx, &path, expr.hir_id);
    }
    if let ExprKind::Unary(UnOp::Not, operand) = expr.kind {
        return Some(!known_boolean_value(cx, operand, remaining_nodes)?);
    }
    if let ExprKind::Binary(operator, left, right) = expr.kind {
        if operator.node == BinOpKind::And {
            if known_boolean_value(cx, left, remaining_nodes)? {
                return known_boolean_value(cx, right, remaining_nodes);
            }
            return Some(false);
        }
        if operator.node == BinOpKind::Or {
            if known_boolean_value(cx, left, remaining_nodes)? {
                return Some(true);
            }
            return known_boolean_value(cx, right, remaining_nodes);
        }
    }
    None
}

/// Evaluate a resolved local boolean `const` item without following runtime values.
fn local_boolean_constant(
    cx: &LateContext<'_>,
    path: &rustc_hir::QPath<'_>,
    hir_id: rustc_hir::HirId,
) -> Option<bool> {
    // Reject locals, associated constants, and external constants before querying CTFE.
    let Res::Def(DefKind::Const { .. }, definition) = cx.typeck_results().qpath_res(path, hir_id)
    else {
        return None;
    };
    if !definition.is_local() {
        return None;
    }

    // A local const must have boolean type before its value can inform the setting.
    let definition_type = cx.tcx.type_of(definition);
    let instantiated_type = definition_type.instantiate_identity();
    let normalized_type = instantiated_type.skip_norm_wip();
    let is_boolean = normalized_type.is_bool();
    if !is_boolean {
        return None;
    }

    // Failed or non-boolean evaluation cannot establish that the setting is `true`.
    cx.tcx
        .const_eval_poly(definition)
        .ok()
        .and_then(|value| value.try_to_bool())
}

/// Return whether an expression has the asynchronous Reqwest `Client` type.
///
/// The check peels references and rejects Reqwest's blocking client namespace so
/// asynchronous client lints cannot confuse the two public APIs.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::is_async_reqwest_client(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_async_reqwest_client(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Peel references before comparing the resolved nominal client type.
    let ty = cx.typeck_results().expr_ty_adjusted(expr).peel_refs();
    let ty::Adt(definition, _) = ty.kind() else {
        return false;
    };
    let def_id = definition.did();

    is_reqwest_def(cx, def_id)
        && cx.tcx.item_name(def_id).as_str() == "Client"
        && !cx.tcx.def_path_str(def_id).contains("::blocking::")
}

/// Return the wrapped value when a call is `Arc::new` or `Rc::new`.
///
/// Resolution requires the standard allocation definitions, which prevents a
/// local constructor named `new` from being treated as reference-count wrapping.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::arc_or_rc_argument(cx, expr);
/// };
/// ```
#[must_use]
pub fn arc_or_rc_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // Match the single-argument constructor shape before resolving its path.
    let ExprKind::Call(callee, [argument]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };
    let path = cx.tcx.def_path_str(def_id);

    // Require alloc's Arc or Rc constructor so local new functions do not match.
    (cx.tcx.crate_name(def_id.krate).as_str() == "alloc"
        && cx.tcx.item_name(def_id).as_str() == "new"
        && (path.contains("::sync::Arc") || path.contains("::rc::Rc")))
    .then_some(argument)
}

/// Return whether an expression names the HTTP `Content-Type` header.
///
/// Both the case-insensitive literal and the canonical `http` constant are
/// supported because Reqwest callers commonly use either representation.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::is_content_type_header(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_content_type_header(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Accept the literal spelling before attempting constant resolution.
    if let ExprKind::Lit(literal) = expr.kind
        && let LitKind::Str(value, _) = literal.node
    {
        return value.as_str().eq_ignore_ascii_case("content-type");
    }

    let ExprKind::Path(ref path) = expr.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, expr.hir_id) else {
        return false;
    };

    // Require the canonical constant from the HTTP crate.
    cx.tcx.item_name(def_id).as_str() == "CONTENT_TYPE"
        && cx.tcx.crate_name(def_id.krate).as_str() == "http"
}

/// Emit one Reqwest diagnostic with actionable help.
///
/// The helper centralizes the diagnostic decorator while keeping each lint's
/// message and replacement guidance explicit at its call site.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, span, message, help| {
///     let _ = reqwest_support::emit_span_lint_with_help(cx, lint, span, message, help);
/// };
/// ```
pub fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic.primary_message(message).help(help);
        }),
    );
}

/// Prove that a definition belongs to Reqwest.
fn is_reqwest_def(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "reqwest"
}

/// Evaluate a statically known `f32` without executing function calls.
///
/// The evaluator follows local non-trait constants and supports unary negation
/// and `+`, `-`, `*`, `/`, and `%` to sixteen levels. It recognizes the core
/// `f32::NAN`, `f32::INFINITY`, and `f32::NEG_INFINITY` constants. Calls, casts,
/// runtime values, and other external constants remain unknown.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = reqwest_support::reqwest_f32_constant(cx, expr);
/// };
/// ```
#[must_use]
pub fn reqwest_f32_constant(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<f32> {
    constant_f32(cx, expr, 0)
}

/// Evaluate one supported f32 expression within the recursion bound.
fn constant_f32(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<f32> {
    // Bound recursion through arithmetic and local constant aliases.
    if depth > 16 {
        return None;
    }

    // Require f32 typing before interpreting literals, operators, or paths.
    let expression_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    if !matches!(expression_type.kind(), ty::Float(ty::FloatTy::F32)) {
        return None;
    }

    // Evaluate only explicitly supported pure syntax.
    if let ExprKind::Lit(literal) = expr.kind {
        if let LitKind::Float(value, _) = literal.node {
            value.as_str().replace('_', "").parse::<f32>().ok()
        } else {
            None
        }
    } else if let ExprKind::Unary(UnOp::Neg, inner) = expr.kind {
        constant_f32(cx, inner, depth + 1).map(|value| -value)
    } else if let ExprKind::Binary(operator, left, right) = expr.kind {
        binary_f32_constant(cx, expr, operator.node, left, right, depth)
    } else if let ExprKind::Path(ref path) = expr.kind {
        path_f32_constant(cx, expr, path, depth)
    } else {
        None
    }
}

/// Evaluate one f32 arithmetic operation when all expression types match.
fn binary_f32_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    operator: BinOpKind,
    left: &Expr<'_>,
    right: &Expr<'_>,
    depth: usize,
) -> Option<f32> {
    // Refuse mixed numeric types rather than approximating Rust conversions.
    let result_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    let left_type = cx.tcx.typeck(left.hir_id.owner.def_id).expr_ty(left);
    let right_type = cx.tcx.typeck(right.hir_id.owner.def_id).expr_ty(right);
    if result_type != left_type || result_type != right_type {
        return None;
    }

    // Evaluate operands at f32 precision before applying the operator.
    let left = constant_f32(cx, left, depth + 1)?;
    let right = constant_f32(cx, right, depth + 1)?;
    match operator {
        BinOpKind::Add => Some(left + right),
        BinOpKind::Sub => Some(left - right),
        BinOpKind::Mul => Some(left * right),
        BinOpKind::Div => Some(left / right),
        BinOpKind::Rem => Some(left % right),
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
    }
}

/// Resolve a local f32 constant or one of core's non-finite f32 constants.
fn path_f32_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    path: &rustc_hir::QPath<'_>,
    depth: usize,
) -> Option<f32> {
    // Resolve names in the body that owns this path, including const initializers.
    let Res::Def(definition_kind, definition) = cx
        .tcx
        .typeck(expr.hir_id.owner.def_id)
        .qpath_res(path, expr.hir_id)
    else {
        return None;
    };

    // Recognize only the documented non-finite f32 constants from core.
    if cx.tcx.crate_name(definition.krate).as_str() == "core" {
        let item_name = cx.tcx.item_name(definition);
        if item_name == Symbol::intern("NAN") {
            return Some(f32::NAN);
        }
        if item_name == Symbol::intern("INFINITY") {
            return Some(f32::INFINITY);
        }
        if item_name == Symbol::intern("NEG_INFINITY") {
            return Some(f32::NEG_INFINITY);
        }
        return None;
    }

    // Follow only local constants and skip trait defaults that implementations can override.
    if !matches!(
        definition_kind,
        DefKind::Const { .. } | DefKind::AssocConst { .. }
    ) || cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait
    {
        return None;
    }
    let local = definition.as_local()?;
    let initializer = cx.tcx.hir_maybe_body_owned_by(local)?.value;

    // Evaluate the initializer in its type-checking owner.
    constant_f32(cx, initializer, depth + 1)
}
