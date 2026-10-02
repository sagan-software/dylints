#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for Axum-specific private lints.
//!
//! The helpers resolve Axum router methods through rustc metadata, validate literal
//! route contracts, and preserve exact source text for machine-applicable fixes.
//! They keep path diagnostics tied to `axum::Router` while excluding local lookalikes.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;

use rustc_ast::LitKind;
use rustc_hir::{Expr, ExprKind, def::Res};
use rustc_lint::{LateContext, LintContext};
use rustc_middle::ty;
use rustc_span::{Span, def_id::DefId};

use dylint_linting as _;

/// A documented invalid literal path accepted by an Axum router method.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouterPathViolation {
    /// The empty string is not a route path.
    /// This variant identifies a source shape that can be replaced with a root path.
    Empty,
    /// Route paths must start with `/`.
    /// This variant identifies a source shape whose missing prefix has an exact fix.
    MissingLeadingSlash,
    /// Axum 0.8 uses `{name}` rather than `:name` captures.
    /// This variant identifies a legacy capture spelling that can be rewritten safely.
    LegacyColonCapture,
    /// Axum 0.8 uses `{*name}` rather than `*name` captures.
    /// This variant identifies a legacy wildcard spelling with a source-preserving fix.
    LegacyWildcardCapture,
    /// Routers cannot be nested at the root.
    /// This variant reports a semantic router constraint that needs user restructuring.
    Root,
    /// Nested router paths cannot contain wildcard captures.
    /// This variant reports a semantic constraint that has no safe literal replacement.
    NestedWildcard,
}

/// One invalid Axum path literal and its optional source-preserving replacement.
///
/// The diagnostic span always covers the complete literal, while the replacement is
/// present only when source spelling and escape boundaries make an exact rewrite safe.
#[derive(Clone, Debug)]
pub struct RouterPathDiagnostic {
    /// Span of the complete path literal used for the primary diagnostic.
    /// The span excludes the surrounding method call and unrelated arguments.
    pub span: Span,
    /// Replacement for the literal when the path rewrite is exact and machine-applicable.
    /// `None` means the lint must provide guidance without changing source text.
    pub replacement: Option<String>,
}

/// One semantically resolved method call on `axum::Router`.
///
/// The borrowed expressions and method span come from the original HIR call, so
/// consumers can inspect arguments without reconstructing a second syntax tree.
#[derive(Clone, Copy, Debug)]
pub struct RouterMethodCall<'hir> {
    /// Receiver expression resolved as an external Axum `Router` value.
    /// The receiver remains borrowed from the analyzed HIR body.
    pub receiver: &'hir Expr<'hir>,
    /// Explicit arguments, excluding the receiver and preserving source order.
    pub arguments: &'hir [Expr<'hir>],
    /// Span of the method identifier used for the primary diagnostic.
    pub method_span: Span,
}

/// Resolve a method call to one exact `axum::Router` method.
///
/// Resolution requires the external Axum crate, the requested method name, and a
/// Router receiver, excluding extension methods and application-defined lookalikes.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method| {
///     let _ = axum_support::router_method_call(cx, expr, expected_method);
/// };
/// ```
pub fn router_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
) -> Option<RouterMethodCall<'hir>> {
    // Reject other expression shapes before consulting type-dependent resolution.
    let ExprKind::MethodCall(segment, receiver, arguments, _) = expr.kind else {
        return None;
    };
    // Require both the resolved Axum method and its Router receiver.
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    if !is_axum_method(cx, def_id, expected_method) || !is_router_type(cx, receiver) {
        return None;
    }

    Some(RouterMethodCall {
        receiver,
        arguments,
        method_span: segment.ident.span,
    })
}

/// Return whether a resolved definition is the named method from the `axum` crate.
fn is_axum_method(cx: &LateContext<'_>, def_id: DefId, expected_method: &str) -> bool {
    // Check the defining crate before comparing the item name.
    let is_axum_item = cx.tcx.crate_name(def_id.krate).as_str() == "axum";
    is_axum_item && cx.tcx.item_name(def_id).as_str() == expected_method
}

/// Return whether an expression has Axum's resolved `Router` type.
///
/// References are peeled before the nominal definition is checked, while unrelated
/// local structs named `Router` remain outside the semantic match.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = axum_support::is_router_type(cx, expr);
/// };
/// ```
pub fn is_router_type(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    let expression_ty = cx.typeck_results().expr_ty(expr).peel_refs();
    let ty::Adt(definition, _) = expression_ty.kind() else {
        return false;
    };

    cx.tcx.crate_name(definition.did().krate).as_str() == "axum"
        && cx.tcx.item_name(definition.did()).as_str() == "Router"
}

/// Return whether an expression is a direct call to `axum::Router::new`.
///
/// The call must have zero arguments and resolve to Axum's constructor before the
/// result type is accepted, preventing same-named local functions from matching.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = axum_support::is_router_new_call(cx, expr);
/// };
/// ```
pub fn is_router_new_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Match a zero-argument call before resolving its callee.
    let ExprKind::Call(callee, []) = expr.kind else {
        return false;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return false;
    };
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, callee.hir_id) else {
        return false;
    };

    // Check the defining crate and result type to reject local lookalikes.
    cx.tcx.crate_name(def_id.krate).as_str() == "axum"
        && cx.tcx.item_name(def_id).as_str() == "new"
        && is_router_type(cx, expr)
}

/// Return the literal path when a resolved router call violates one path contract.
///
/// Only direct string literals are eligible for fixes. Computed paths still receive
/// semantic diagnostics when their source shape cannot be rewritten without guessing.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method, violation| {
///     let _ = axum_support::router_path_violation(cx, expr, expected_method, violation);
/// };
/// ```
pub fn router_path_violation(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected_method: &str,
    violation: RouterPathViolation,
) -> Option<RouterPathDiagnostic> {
    // Restrict literal validation to the resolved Router method.
    let (path, value) = router_literal_path(cx, expr, expected_method)?;
    // Borrow the interned text before applying the selected path contract.
    let value = value.as_str();

    if !is_path_violation(value, violation) {
        return None;
    }

    // Limit machine fixes to literals whose source spelling can be rewritten without
    // guessing at escape sequences or raw-string delimiters.
    let replacement = cx
        .sess()
        .source_map()
        .span_to_snippet(path.span)
        .ok()
        .and_then(|source| path_replacement(source.as_str(), value, violation));

    Some(RouterPathDiagnostic {
        span: path.span,
        replacement,
    })
}

/// Resolve one direct string-literal path argument from an Axum router call.
fn router_literal_path<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
) -> Option<(&'hir Expr<'hir>, rustc_span::Symbol)> {
    let call = router_method_call(cx, expr, expected_method)?;
    let path = call.arguments.first()?;
    let ExprKind::Lit(literal) = path.kind else {
        return None;
    };
    let LitKind::Str(value, _) = literal.node else {
        return None;
    };
    Some((path, value))
}

/// Return whether a router path literal value breaks the selected path contract.
fn is_path_violation(value: &str, violation: RouterPathViolation) -> bool {
    // Each variant is disjoint where rules overlap, so one literal has one diagnostic.
    match violation {
        RouterPathViolation::Empty => value.is_empty(),
        RouterPathViolation::MissingLeadingSlash => !value.is_empty() && !value.starts_with('/'),
        RouterPathViolation::LegacyColonCapture => {
            value.split('/').any(|segment| segment.starts_with(':'))
        }
        RouterPathViolation::LegacyWildcardCapture => {
            value.split('/').any(|segment| segment.starts_with('*'))
        }
        RouterPathViolation::Root => {
            value.is_empty() || (value.len() == 1 && value.as_bytes() == *b"/")
        }
        RouterPathViolation::NestedWildcard => value.contains("{*"),
    }
}

/// Build a machine-applicable replacement for one simple path literal.
fn path_replacement(source: &str, value: &str, violation: RouterPathViolation) -> Option<String> {
    let contents = literal_contents(source)?;
    if contents != value {
        return None;
    }

    let replacement = path_replacement_text(value, violation)?;
    Some(format!("\"{replacement}\""))
}

/// Return the unescaped contents of a simple cooked string source literal.
fn literal_contents(source: &str) -> Option<&str> {
    let source = source.trim();
    source.strip_prefix('"')?.strip_suffix('"')
}

/// Build the replacement text after the source literal has been validated.
fn path_replacement_text(value: &str, violation: RouterPathViolation) -> Option<String> {
    Some(match violation {
        RouterPathViolation::Empty => char::from(b'/').to_string(),
        RouterPathViolation::MissingLeadingSlash => format!("{}{value}", char::from(b'/')),
        RouterPathViolation::LegacyColonCapture => replace_legacy_captures(value, ':', "{", "}")?,
        RouterPathViolation::LegacyWildcardCapture => {
            replace_legacy_captures(value, '*', "{*", "}")?
        }
        RouterPathViolation::Root | RouterPathViolation::NestedWildcard => return None,
    })
}

/// Convert valid legacy capture segments while preserving every other path segment.
fn replace_legacy_captures(
    value: &str,
    prefix: char,
    opening: &str,
    closing: &str,
) -> Option<String> {
    // Build one replacement while preserving every non-capture segment.
    let mut is_changed = false;
    let mut replacement = String::with_capacity(value.len() + 4);
    // Validate each capture name before emitting its brace syntax.
    for (index, segment) in value.split('/').enumerate() {
        if index > 0 {
            replacement.push('/');
        }
        if let Some(name) = segment.strip_prefix(prefix) {
            if !valid_capture_name(name) {
                return None;
            }
            replacement.push_str(opening);
            replacement.push_str(name);
            replacement.push_str(closing);
            is_changed = true;
        } else {
            replacement.push_str(segment);
        }
    }

    // Return no suggestion when the selected legacy prefix was not present.
    is_changed.then_some(replacement)
}

/// Return whether a route capture name can be carried into Axum's brace syntax.
fn valid_capture_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first == '_' || first.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

/// Declare one Axum literal-path contract as a documented Dylint lint.
#[macro_export]
macro_rules! declare_router_path_lint {
    (
        $lint:ident,
        $pass:ident,
        $method:literal,
        $violation:ident,
        $description:literal,
        $message:literal,
        $help:literal
    ) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check a resolved Axum router call and its literal path argument.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(path_diagnostic) = $crate::router_path_violation(
                    cx,
                    expr,
                    $method,
                    $crate::RouterPathViolation::$violation,
                ) else {
                    return;
                };

                cx.emit_span_lint(
                    $lint,
                    path_diagnostic.span,
                    rustc_errors::DiagDecorator(move |diagnostic| {
                        let _configured_diagnostic = diagnostic.primary_message($message);
                        if let Some(replacement) = path_diagnostic.replacement {
                            let _configured_suggestion = diagnostic.span_suggestion(
                                path_diagnostic.span,
                                $help,
                                replacement,
                                rustc_errors::Applicability::MachineApplicable,
                            );
                        } else {
                            let _configured_help = diagnostic.help($help);
                        }
                    }),
                );
            }
        }

        /// Run the UI fixture.
        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::{RouterPathViolation, path_replacement};

    #[test]
    fn path_replacement_handles_exact_literal_rewrites() {
        let cases = [
            ("\"\"", RouterPathViolation::Empty),
            ("\"users/:id\"", RouterPathViolation::MissingLeadingSlash),
            ("\"/users/:id\"", RouterPathViolation::LegacyColonCapture),
            (
                "\"/assets/*path/:name\"",
                RouterPathViolation::LegacyWildcardCapture,
            ),
        ];

        assert_eq!(
            replacements(&cases),
            [
                Some("\"/\"".to_owned()),
                Some("\"/users/:id\"".to_owned()),
                Some("\"/users/{id}\"".to_owned()),
                Some("\"/assets/{*path}/:name\"".to_owned()),
            ]
        );
    }

    #[test]
    fn path_replacement_rejects_ambiguous_source_shapes() {
        let cases = [
            ("r#\"/users/:id\"#", RouterPathViolation::LegacyColonCapture),
            (
                "\"/users/:id-name\"",
                RouterPathViolation::LegacyColonCapture,
            ),
            ("\"/users/:id\"", RouterPathViolation::Root),
            ("\"/users/{*path}\"", RouterPathViolation::NestedWildcard),
        ];

        assert_eq!(replacements(&cases), [None, None, None, None]);
    }

    /// Apply `path_replacement` to each source literal with the value it spells.
    fn replacements(cases: &[(&str, RouterPathViolation)]) -> Vec<Option<String>> {
        cases
            .iter()
            .map(|&(source, violation)| path_replacement(source, literal_value(source), violation))
            .collect()
    }

    /// Return the value spelled by a cooked or raw string literal without escapes.
    fn literal_value(source: &str) -> &str {
        source
            .trim_start_matches('r')
            .trim_matches('#')
            .trim_matches('"')
    }
}
