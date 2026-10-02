#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for Tokio-specific private lints.
//!
//! These functions resolve Tokio methods and free functions through rustc's
//! semantic metadata, then expose small typed results for individual lint rules.
//! They deliberately reject local lookalikes and preserve source spans for fixes.
//! Callers can therefore compose the results into focused diagnostics without
//! depending on Tokio's private implementation modules or runtime behavior.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_hir::{ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, Node, def::Res};
use rustc_lint::LateContext;
use rustc_span::{Span, Symbol, def_id::DefId};

use dylint_linting as _;

/// One semantically resolved Tokio method call with its canonical definition path.
///
/// The value preserves the method span for diagnostics and the resolved path for
/// callers that need to distinguish public Tokio APIs from local extensions.
#[derive(Clone, Debug)]
pub struct TokioMethod {
    /// User-facing method-name span used for the primary diagnostic and source fix.
    /// The span always identifies the method token rather than the whole call.
    pub span: Span,
    /// Resolved method name obtained from rustc's type-dependent method lookup.
    /// The symbol identifies the external Tokio method selected by the compiler.
    pub name: Symbol,
    /// Canonical Rust definition path used to identify the Tokio operation.
    /// The path distinguishes public re-exports from local methods with matching names.
    pub definition: String,
}

/// Return the arguments when an expression calls the target Tokio function.
///
/// The helper accepts only direct path calls whose resolved definition matches
/// the expected Tokio path, so aliases and unrelated functions do not leak in.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_path| {
///     let _ = tokio_support::tokio_function_arguments(cx, expr, expected_path);
/// };
/// ```
pub fn tokio_function_arguments<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_path: &str,
) -> Option<&'hir [Expr<'hir>]> {
    // Resolve the callee before comparing its canonical Tokio definition.
    let ExprKind::Call(callee, arguments) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };

    // Return arguments only when the resolved definition has the exact Tokio path.
    is_tokio_def(cx, def_id, expected_path).then_some(arguments)
}

/// Resolve a method call only when the method is defined by Tokio.
///
/// The returned value combines the user-facing span with canonical metadata so
/// each lint can report the call without repeating rustc resolution logic.
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
    (cx.tcx.crate_name(def_id.krate).as_str() == "tokio").then_some(TokioMethod {
        span: segment.ident.span,
        name: segment.ident.name,
        definition: cx.tcx.def_path_str(def_id),
    })
}

/// Return the callee span and arguments for one resolved Tokio function.
///
/// This lower-level result is useful when a lint needs both the call location and
/// the original arguments while retaining the same exact-definition guarantee.
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
    // Preserve the callee span and arguments only for the exact Tokio function.
    is_tokio_def(cx, def_id, expected_path).then_some((callee.span, arguments))
}

/// Return whether a closure is async or synchronously returns an async block.
///
/// The check follows compiler coroutine representations and nested expression
/// wrappers instead of relying on source spelling or the closure's inferred name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = tokio_support::is_async_closure(cx, expr);
/// };
/// ```
pub fn is_async_closure(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let ExprKind::Closure(closure) = expr.kind else {
        return false;
    };
    matches!(
        closure.kind,
        ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
            | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
    ) || is_async_expression(cx.tcx.hir_body(closure.body).value)
}

/// Strip compiler wrappers before checking for an async block expression.
fn is_async_expression(expr: &Expr<'_>) -> bool {
    // Recognize closure nodes before peeling compiler-generated wrappers.
    if let ExprKind::Closure(closure) = expr.kind {
        return matches!(
            closure.kind,
            ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
                | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
        );
    }
    // Peel transient and block wrappers recursively to reach the user expression.
    if let ExprKind::DropTemps(inner) = expr.kind {
        return is_async_expression(inner);
    }
    if let ExprKind::Block(block, _) = expr.kind {
        return block.expr.is_some_and(is_async_expression);
    }
    false
}

/// Return whether an expression is lexically nested in a loop.
///
/// Parent traversal stops only at the enclosing HIR root, so a caller can use the
/// result to distinguish loop-local Tokio operations from equivalent top-level calls.
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
    cx.tcx.hir_parent_iter(expr.hir_id).any(
        |(_, node)| matches!(node, Node::Expr(parent) if matches!(parent.kind, ExprKind::Loop(..))),
    )
}

/// Return whether an expression is an integer literal equal to zero.
///
/// Only an integer literal with the exact numeric value zero matches; names,
/// casts, and computed expressions remain outside this deliberately narrow helper.
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

/// Recognize common compile-time spellings of a zero `Duration`.
///
/// The helper accepts the standard associated constant and zero-valued constructors
/// only after rustc confirms that the definition belongs to `core::time::Duration`.
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
    // First recognize the canonical associated constant through name resolution.
    if let ExprKind::Path(ref path) = expr.kind
        && let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, expr.hir_id)
    {
        return is_duration_def(cx, def_id, "ZERO");
    }

    // Then handle standard constructors only when they resolve to `Duration`.
    let ExprKind::Call(callee, arguments) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return false;
    };

    // Match the closed constructor set and require every duration component to be zero.
    match cx.tcx.item_name(def_id).as_str() {
        "from_secs" | "from_millis" | "from_micros" | "from_nanos"
            if is_duration_def(cx, def_id, cx.tcx.item_name(def_id).as_str()) =>
        {
            matches!(arguments, [argument] if is_zero_integer(argument))
        }
        "new" if is_duration_def(cx, def_id, "new") => {
            matches!(arguments, [seconds, nanos] if is_zero_integer(seconds) && is_zero_integer(nanos))
        }
        _ => false,
    }
}

/// Return whether the expression runs in the nearest async body.
///
/// The nearest closure boundary controls the result, which prevents a synchronous
/// nested closure from inheriting the async state of its surrounding function.
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
    // Stop at the nearest closure so a `spawn_blocking` closure remains synchronous.
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        let Node::Expr(parent) = node else {
            continue;
        };
        let ExprKind::Closure(closure) = parent.kind else {
            continue;
        };

        // The nearest closure boundary decides whether this call itself runs asynchronously.
        return matches!(
            closure.kind,
            ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
                | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
        );
    }

    false
}

/// Prove that a definition is the target Tokio item.
fn is_tokio_def(cx: &LateContext<'_>, def_id: DefId, expected_path: &str) -> bool {
    if cx.tcx.crate_name(def_id.krate).as_str() != "tokio" {
        return false;
    }

    // Some public re-exports omit private implementation modules from the displayed path.
    let path = cx.tcx.def_path_str(def_id);
    path == expected_path || is_tokio_reexport(cx, def_id, expected_path, &path)
}

/// Prove that a definition is a supported Tokio public re-export.
fn is_tokio_reexport(cx: &LateContext<'_>, def_id: DefId, expected_path: &str, path: &str) -> bool {
    // Select the re-export rule for the requested public spelling.
    let Some(reexport) = TOKIO_REEXPORTS
        .iter()
        .find(|reexport| reexport.public_spelling == expected_path)
    else {
        return false;
    };

    // Require the stable item name and every private-module fragment in the displayed path.
    let is_expected_item = cx.tcx.item_name(def_id).as_str() == reexport.item_name;
    is_expected_item
        && reexport
            .path_fragments
            .iter()
            .all(|fragment| path.contains(fragment))
}

/// A public Tokio path whose displayed definition path can name private modules.
struct TokioReexport {
    /// Public definition-path spelling requested by lint callers.
    public_spelling: &'static str,
    /// Stable item name of the re-exported definition.
    item_name: &'static str,
    /// Fragments that the displayed definition path must contain.
    path_fragments: &'static [&'static str],
}

/// Supported Tokio re-exports, keyed by their public path spelling.
const TOKIO_REEXPORTS: &[TokioReexport] = &[
    TokioReexport {
        public_spelling: "tokio::sync::mpsc::bounded::channel",
        item_name: "channel",
        path_fragments: &["::sync::mpsc::"],
    },
    TokioReexport {
        public_spelling: "tokio::sync::mpsc::unbounded::unbounded_channel",
        item_name: "unbounded_channel",
        path_fragments: &["::sync::mpsc::"],
    },
    TokioReexport {
        public_spelling: "tokio::task::blocking::spawn_blocking",
        item_name: "spawn_blocking",
        path_fragments: &[],
    },
    TokioReexport {
        public_spelling: "tokio::runtime::runtime::Runtime::new",
        item_name: "new",
        path_fragments: &["::runtime::", "Runtime::new"],
    },
    TokioReexport {
        public_spelling: "tokio::time::sleep::sleep",
        item_name: "sleep",
        path_fragments: &["::time::sleep"],
    },
    TokioReexport {
        public_spelling: "tokio::time::interval::interval",
        item_name: "interval",
        path_fragments: &["::time::"],
    },
    TokioReexport {
        public_spelling: "tokio::time::interval::interval_at",
        item_name: "interval_at",
        path_fragments: &["::time::"],
    },
];

/// Prove that a definition is one item on `core::time::Duration`.
fn is_duration_def(cx: &LateContext<'_>, def_id: DefId, item_name: &str) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "core"
        && cx.tcx.item_name(def_id).as_str() == item_name
        && cx.tcx.def_path_str(def_id).contains("time::Duration::")
}
