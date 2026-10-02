#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Tokio-specific private lints.
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
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, Node, def::Res};
use rustc_lint::{LateContext, Lint, LintContext as _};
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::tokio_method(cx, expr);
/// };
/// ```
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_path| {
///     let _ = tokio_support::tokio_function_call(cx, expr, expected_path);
/// };
/// ```
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_in_loop(cx, expr);
/// };
/// ```
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |expr| {
///     let _ = tokio_support::is_zero_integer(expr);
/// };
/// ```
pub fn is_zero_integer(expr: &Expr<'_>) -> bool {
    matches!(expr.kind, ExprKind::Lit(literal) if matches!(literal.node, LitKind::Int(value, _) if value.get() == 0))
}

/// Recognize the compile-time spellings of a zero `core::time::Duration`.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_zero_duration(cx, expr);
/// };
/// ```
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_in_async_body(cx, expr);
/// };
/// ```
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
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::await_suggestion(cx, expr);
/// };
/// ```
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
