#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Tokio-specific lints.
//!
//! These functions resolve Tokio methods and free functions to their definition
//! paths, which name the defining module rather than any re-export, and expose
//! small typed results for individual lint rules. Local items with the same
//! names never match. Context helpers stop at the nearest closure or item
//! boundary, so code that runs in a different body does not inherit an async
//! or loop context.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    BinOpKind, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, Node,
    def::{DefKind, Res},
};
use rustc_lint::{LateContext, Lint, LintContext as _};
use rustc_middle::ty::{self, UintTy};
use rustc_span::{Span, Symbol, def_id::DefId};

use dylint_linting as _;

/// One method call that resolves to a Tokio definition.
#[derive(Clone, Debug)]
pub struct TokioMethod {
    /// The method-name span, used for the primary diagnostic.
    pub span: Span,
    /// The resolved method name.
    pub name: Symbol,
    /// The resolved definition path, such as `tokio::runtime::handle::Handle::block_on`.
    pub definition_name: String,
}

/// Resolve a method call only when the method is defined by Tokio.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::tokio_method(cx, expr);
/// };
/// ```
#[must_use]
pub fn tokio_method(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<TokioMethod> {
    // Type-dependent resolution rejects extension traits and user methods with the same spelling.
    let ExprKind::MethodCall(segment, ..) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    (cx.tcx.crate_name(def_id.krate).as_str() == "tokio").then(|| TokioMethod {
        span: segment.ident.span,
        name: segment.ident.name,
        definition_name: def_path(cx, def_id),
    })
}

/// Return the callee span and arguments of a call to the Tokio function at `expected_path`.
///
/// `expected_path` is the definition path, which names the defining module,
/// such as `tokio::sync::mpsc::bounded::channel` for `tokio::sync::mpsc::channel`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_path| {
///     let _ = tokio_support::tokio_function_call(cx, expr, expected_path);
/// };
/// ```
#[must_use]
pub fn tokio_function_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_path: &str,
) -> Option<(Span, &'hir [Expr<'hir>])> {
    // Require a direct path call before asking rustc for its definition.
    let ExprKind::Call(callee, arguments) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };
    (def_path(cx, def_id) == expected_path).then_some((callee.span, arguments))
}

/// Return the definition path of an item, joined with `::`.
fn def_path(cx: &LateContext<'_>, def_id: DefId) -> String {
    cx.get_def_path(def_id)
        .iter()
        .map(Symbol::as_str)
        .collect::<Vec<_>>()
        .join("::")
}

/// Return whether an expression runs inside a loop body of its own function or closure.
///
/// The search stops at the nearest closure or item, so an async block spawned
/// from a loop is not itself in that loop.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_in_loop(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        if let Node::Expr(parent) = node {
            if matches!(parent.kind, ExprKind::Loop(..)) {
                return true;
            }
            if matches!(parent.kind, ExprKind::Closure(_)) {
                return false;
            }
        }
        if matches!(node, Node::Item(_) | Node::ImplItem(_) | Node::TraitItem(_)) {
            return false;
        }
    }
    false
}

/// Return whether an expression is an integer literal equal to zero.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |expr| {
///     let _ = tokio_support::is_zero_integer(expr);
/// };
/// ```
#[must_use]
pub fn is_zero_integer(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal) if matches!(literal.node, LitKind::Int(value, _) if value.get() == 0))
}

/// Recognize the compile-time spellings of a zero `core::time::Duration`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_zero_duration(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_zero_duration(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // First recognize the associated constant through name resolution.
    if let ExprKind::Path(ref path) = expr.kind
        && let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, expr.hir_id)
    {
        return def_path(cx, def_id) == "core::time::Duration::ZERO";
    }

    // Then require a standard constructor with every component zero.
    let ExprKind::Call(callee, arguments) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return false;
    };
    match def_path(cx, def_id).as_str() {
        "core::time::Duration::from_secs"
        | "core::time::Duration::from_millis"
        | "core::time::Duration::from_micros"
        | "core::time::Duration::from_nanos" => {
            matches!(arguments, [argument] if is_zero_integer(argument))
        }
        "core::time::Duration::new" => {
            matches!(arguments, [seconds, nanos] if is_zero_integer(seconds) && is_zero_integer(nanos))
        }
        _ => false,
    }
}

/// Return whether the expression runs in an async body.
///
/// The nearest closure decides: an async block, async closure, or async
/// function body is async, and a plain closure is not. A nested item is never
/// async through its parent.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_in_async_body(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_in_async_body(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        if let Node::Expr(Expr {
            kind: ExprKind::Closure(closure),
            ..
        }) = node
        {
            return matches!(
                closure.kind,
                ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
                    | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
            );
        }
        if matches!(node, Node::Item(_) | Node::ImplItem(_) | Node::TraitItem(_)) {
            return false;
        }
    }
    false
}

/// Suggest replacing a `block_on(future)` method call with `future.await`.
///
/// The suggestion is `MaybeIncorrect`: the enclosing future gains an await
/// point, which can change whether it is `Send`.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::await_suggestion(cx, expr);
/// };
/// ```
#[must_use]
pub fn await_suggestion(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<(Span, String, Applicability)> {
    let ExprKind::MethodCall(_, _, [future], _) = expr.kind else {
        return None;
    };
    let snippet = cx
        .sess()
        .source_map()
        .span_to_snippet(future.span.source_callsite())
        .ok()?;
    // `.await` binds tighter than operators, so wrap anything but a postfix-safe expression.
    let needs_parens = !matches!(
        future.kind,
        ExprKind::Path(..)
            | ExprKind::Call(..)
            | ExprKind::MethodCall(..)
            | ExprKind::Block(..)
            | ExprKind::Closure(..)
            | ExprKind::Field(..)
    ) && !future.span.from_expansion();
    let replacement = if needs_parens {
        format!("({snippet}).await")
    } else {
        format!("{snippet}.await")
    };
    Some((expr.span, replacement, Applicability::MaybeIncorrect))
}

/// Emit one diagnostic with either a suggestion or a help message.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, lint, span| {
///     tokio_support::emit(cx, lint, span, "message", "help", None);
/// };
/// ```
pub fn emit(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
    suggestion: Option<(Span, String, Applicability)>,
) {
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diagnostic| {
            let diagnostic = diagnostic.primary_message(message);
            match suggestion {
                Some((span, replacement, applicability)) => {
                    let _configured_suggestion =
                        diagnostic.span_suggestion(span, help, replacement, applicability);
                }
                None => {
                    let _configured_help = diagnostic.help(help);
                }
            }
        }),
    );
}

/// Return whether an unsigned integer expression has a statically known zero value.
///
/// The evaluator follows local non-trait constants and supports `+`, `-`, `*`,
/// `/`, and `%` to sixteen levels. It skips calls, casts, external constants,
/// runtime values, and operations that overflow or underflow their integer type.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_zero_integer_constant(cx, expr);
/// };
/// ```
#[must_use]
pub fn is_zero_integer_constant(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    integer_constant(cx, expr, 0) == Some(0)
}

/// Evaluate one supported unsigned integer expression without executing code.
fn integer_constant(cx: &LateContext<'_>, expr: &Expr<'_>, depth: usize) -> Option<u128> {
    // Bound recursion through arithmetic and nested constant definitions.
    if depth > 16 {
        return None;
    }

    // Resolve the expression's integer width before interpreting its syntax.
    let integer_maximum = unsigned_integer_maximum(cx, expr)?;

    // Evaluate only unsigned literals, supported arithmetic, and constant paths.
    let value = if let ExprKind::Lit(literal) = expr.kind {
        if let LitKind::Int(value, _) = literal.node {
            Some(value.get())
        } else {
            None
        }
    } else if let ExprKind::Binary(operator, left, right) = expr.kind {
        binary_integer_constant(cx, expr, operator.node, left, right, depth)
    } else if let ExprKind::Path(ref path) = expr.kind {
        path_integer_constant(cx, expr, path, depth)
    } else {
        None
    }?;

    // Refuse results outside the expression's unsigned type range.
    (value <= integer_maximum).then_some(value)
}

/// Return the maximum value of an unsigned integer expression's type.
fn unsigned_integer_maximum(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<u128> {
    // Read type information from the body that owns this expression.
    let expression_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    let ty::Uint(integer_type) = expression_type.kind() else {
        return None;
    };

    // Use the target pointer width for `usize` and fixed widths for other types.
    let width = match integer_type {
        UintTy::U8 => 8,
        UintTy::U16 => 16,
        UintTy::U32 => 32,
        UintTy::U64 => 64,
        UintTy::U128 => 128,
        UintTy::Usize => cx.tcx.data_layout.pointer_size().bits(),
    };
    if width == u64::from(u128::BITS) {
        Some(u128::MAX)
    } else {
        Some((1_u128 << u32::try_from(width).ok()?) - 1)
    }
}

/// Evaluate supported arithmetic after checking operand types and widths.
fn binary_integer_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    operator: BinOpKind,
    left: &Expr<'_>,
    right: &Expr<'_>,
    depth: usize,
) -> Option<u128> {
    // Require matching unsigned expression types before applying an operator.
    let result_type = cx.tcx.typeck(expr.hir_id.owner.def_id).expr_ty(expr);
    let left_type = cx.tcx.typeck(left.hir_id.owner.def_id).expr_ty(left);
    let right_type = cx.tcx.typeck(right.hir_id.owner.def_id).expr_ty(right);
    if result_type != left_type || result_type != right_type {
        return None;
    }

    // Evaluate operands before applying checked arithmetic.
    let left = integer_constant(cx, left, depth + 1)?;
    let right = integer_constant(cx, right, depth + 1)?;
    match operator {
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
    }
}

/// Resolve and evaluate a local non-trait integer constant path.
fn path_integer_constant(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    path: &rustc_hir::QPath<'_>,
    depth: usize,
) -> Option<u128> {
    // Accept only constant definitions and exclude trait defaults that can vary by implementation.
    let Res::Def(DefKind::Const { .. } | DefKind::AssocConst { .. }, definition) = cx
        .tcx
        .typeck(expr.hir_id.owner.def_id)
        .qpath_res(path, expr.hir_id)
    else {
        return None;
    };
    if cx.tcx.def_kind(cx.tcx.parent(definition)) == DefKind::Trait {
        return None;
    }
    let local = definition.as_local()?;
    let initializer = cx.tcx.hir_maybe_body_owned_by(local)?.value;

    // Evaluate the initializer in its own body and type context.
    integer_constant(cx, initializer, depth + 1)
}
