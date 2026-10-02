#![feature(rustc_private)]
#![doc(hidden)]

//! Shared semantic helpers for `SQLx`-specific private lints.
//!
//! These helpers resolve `SQLx` methods, constructors, and macro expansions with
//! rustc metadata. They preserve source spans and reject local lookalikes so
//! each private lint can focus on one documented `SQLx` contract without duplicating
//! compiler-resolution code across its implementation.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind, def::Res};
use rustc_lint::{LateContext, LintContext as _};
use rustc_span::{ExpnKind, MacroKind, Span, Symbol, def_id::DefId};

use dylint_linting as _;

/// One semantically resolved `SQLx` method call and its original arguments.
///
/// The value is borrowed from HIR and retains the exact method span needed for a
/// diagnostic while preserving the resolved name for the individual lint rule.
#[derive(Clone, Copy, Debug)]
pub struct SqlxMethodCall<'hir> {
    /// User-facing method-name span used for the primary diagnostic.
    pub span: Span,
    /// Resolved method name obtained from type-dependent lookup.
    pub name: Symbol,
    /// Explicit method arguments borrowed from the original HIR expression.
    pub arguments: &'hir [Expr<'hir>],
}

/// One semantically resolved `SQLx` macro call and its source location.
///
/// Macro expansion metadata is retained so callers can diagnose the public `SQLx`
/// spelling even when its implementation delegates through another macro.
#[derive(Clone, Copy, Debug)]
pub struct SqlxMacroCall {
    /// User-facing macro invocation span used for the primary diagnostic.
    pub span: Span,
    /// Resolved macro name obtained from expansion metadata.
    pub name: Symbol,
}

/// Resolve a method call to an exact `SQLx` owner and one allowed method name.
///
/// Type-dependent resolution and owner matching exclude extension traits, local
/// methods, and unrelated APIs that happen to share the same source spelling.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_owner, expected_methods| {
///     let _ = sqlx_support::sqlx_method_call(cx, expr, expected_owner, expected_methods);
/// };
/// ```
pub fn sqlx_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_owner: &str,
    expected_methods: &[&str],
) -> Option<SqlxMethodCall<'hir>> {
    // Type-dependent resolution rejects extension traits and user methods with the same spelling.
    let ExprKind::MethodCall(segment, _, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    let method_name = segment.ident.name;

    (is_sqlx_def(cx, def_id)
        && expected_methods.contains(&method_name.as_str())
        && has_definition_owner(cx, def_id, expected_owner))
    .then_some(SqlxMethodCall {
        span: segment.ident.span,
        name: method_name,
        arguments,
    })
}

/// A documented argument shape for a `SQLx` method lint.
///
/// Each variant describes one closed source pattern so the caller can select a
/// precise diagnostic without embedding argument-shape matching in every lint.
#[derive(Clone, Copy, Debug)]
pub enum MethodArgumentViolation {
    /// Every resolved call is discouraged by the selected `SQLx` lint rule.
    Any,
    /// The first argument is built with `format!`, producing a dynamic SQL string.
    FormattedSql,
    /// The first argument is an empty collection literal with no query values.
    EmptyCollection,
    /// The first argument is integer zero, which represents an invalid bound value.
    Zero,
}

/// Match a `SQLx` method call with one argument violation.
///
/// The returned call is available only when the owner, method name, and selected
/// argument pattern all match the resolved `SQLx` API contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, owner, expected_method, violation| {
///     let _ = sqlx_support::sqlx_method_argument_violation(cx, expr, owner, expected_method, violation);
/// };
/// ```
pub fn sqlx_method_argument_violation<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    owner: &str,
    expected_method: &str,
    violation: MethodArgumentViolation,
) -> Option<SqlxMethodCall<'hir>> {
    let call = sqlx_method_call(cx, expr, owner, &[expected_method])?;
    let is_invalid = match violation {
        MethodArgumentViolation::Any => true,
        MethodArgumentViolation::FormattedSql => call.arguments.first().is_some_and(|argument| {
            cx.sess()
                .source_map()
                .span_to_snippet(argument.span.source_callsite())
                .is_ok_and(|snippet| snippet.trim_start().starts_with("format!"))
        }),
        MethodArgumentViolation::EmptyCollection => matches!(
            call.arguments,
            [argument, ..] if matches!(argument.kind, ExprKind::Array([]))
        ),
        MethodArgumentViolation::Zero => matches!(
            call.arguments,
            [argument, ..]
                if matches!(
                    argument.kind,
                    ExprKind::Lit(literal)
                        if matches!(literal.node, LitKind::Int(value, _) if value.get() == 0)
                )
        ),
    };
    is_invalid.then_some(call)
}

/// Declare one `SQLx` method-argument lint.
#[macro_export]
macro_rules! declare_method_argument_lint {
    (
        $lint:ident, $pass:ident, $owner:literal, $method:literal, $violation:ident,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }
        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check the resolved SQLx owner, method, and argument shape.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(call) = $crate::sqlx_method_argument_violation(
                    cx,
                    expr,
                    $owner,
                    $method,
                    $crate::MethodArgumentViolation::$violation,
                ) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic =
                            diagnostic.primary_message($message).help($help);
                    }),
                );
            }
        }
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return the argument when an expression constructs `SQLx`'s `AssertSqlSafe`.
///
/// The constructor is resolved semantically, allowing aliases while excluding
/// local tuple constructors that merely reuse the same type name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = sqlx_support::assert_sql_safe_argument(cx, expr);
/// };
/// ```
pub fn assert_sql_safe_argument<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
) -> Option<&'hir Expr<'hir>> {
    // Resolve the tuple-struct constructor so aliases still match and local lookalikes do not.
    let ExprKind::Call(callee, [argument]) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return None;
    };
    // Compare the resolved definition path after the constructor shape is known.
    let definition_path = cx.tcx.def_path_str(def_id);

    (is_sqlx_def(cx, def_id)
        && definition_path
            .split("::")
            .any(|segment| segment == "AssertSqlSafe"))
    .then_some(argument)
}

/// Resolve an unchecked `SQLx` query macro from an expanded expression.
///
/// Expansion traversal follows `SQLx`'s public macro into its implementation until
/// it finds the unchecked definition, preserving the public call-site span.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, span| {
///     let _ = sqlx_support::unchecked_query_macro(cx, span);
/// };
/// ```
pub fn unchecked_query_macro(cx: &LateContext<'_>, span: Span) -> Option<SqlxMacroCall> {
    let mut expansion = span.ctxt().outer_expn_data();

    // Walk outward because SQLx's public macro delegates to its proc-macro implementation.
    loop {
        if matches!(expansion.kind, ExpnKind::Macro(MacroKind::Bang, _))
            && let Some(def_id) = expansion.macro_def_id
            && is_sqlx_def(cx, def_id)
            && is_unchecked_query_macro(cx.tcx.item_name(def_id).as_str())
        {
            return Some(SqlxMacroCall {
                span: expansion.call_site,
                name: cx.tcx.item_name(def_id),
            });
        }

        if !expansion.call_site.from_expansion() {
            return None;
        }
        // Continue with the caller expansion until the public SQLx macro is found.
        expansion = expansion.call_site.ctxt().outer_expn_data();
    }
}

/// Return the checked counterpart for an unchecked `SQLx` query macro.
///
/// The conversion removes only `SQLx`'s `_unchecked` suffix and returns `None` for
/// names that do not use that exact macro naming convention.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |name| {
///     let _ = sqlx_support::checked_query_macro_name(name);
/// };
/// ```
pub fn checked_query_macro_name(name: &str) -> Option<&str> {
    name.strip_suffix("_unchecked")
}

/// Prove that a definition comes from `SQLx`'s facade or core crate.
fn is_sqlx_def(cx: &LateContext<'_>, def_id: DefId) -> bool {
    matches!(
        cx.tcx.crate_name(def_id.krate).as_str(),
        "sqlx" | "sqlx_core"
    )
}

/// Return whether the segment immediately before the item name is the expected owner.
fn has_definition_owner(cx: &LateContext<'_>, def_id: DefId, expected_owner: &str) -> bool {
    let path = cx.tcx.def_path_str(def_id);
    path.rsplit("::").nth(1) == Some(expected_owner)
}

/// Return whether the macro skips `SQLx`'s input or output type checking.
fn is_unchecked_query_macro(name: &str) -> bool {
    matches!(
        name,
        "query_unchecked"
            | "query_as_unchecked"
            | "query_file_unchecked"
            | "query_file_as_unchecked"
            | "query_scalar_unchecked"
            | "query_file_scalar_unchecked"
    )
}
