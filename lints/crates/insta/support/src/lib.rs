#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![doc(hidden)]

//! Shared semantic helpers for Insta-specific private lints.
//!
//! These helpers resolve Insta macros and methods through rustc metadata, recover
//! source-preserving arguments, and expose small results for individual lint rules.
//! They keep snapshot diagnostics precise while avoiding guesses based only on names.

extern crate rustc_ast;
extern crate rustc_driver as _;
extern crate rustc_hir;
extern crate rustc_infer;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_span;
extern crate rustc_trait_selection;

use dylint_linting as _;
use rustc_ast::LitKind;
use rustc_hir::{
    BindingMode, ClosureKind, CoroutineDesugaring, CoroutineKind, Expr, ExprKind, Node, PatKind,
    QPath,
    def::{DefKind, Res},
};
use rustc_infer::infer::TyCtxtInferExt as _;
use rustc_lint::{LateContext, LintContext};
use rustc_middle::ty::Ty;
use rustc_span::{ExpnData, ExpnKind, MacroKind, Span, SyntaxContext, def_id::DefId, sym};
use rustc_trait_selection::infer::InferCtxtExt as _;

/// Public Insta snapshot assertion macros.
const SNAPSHOT_MACROS: &[&str] = &[
    "assert_binary_snapshot",
    "assert_compact_debug_snapshot",
    "assert_compact_json_snapshot",
    "assert_csv_snapshot",
    "assert_debug_snapshot",
    "assert_display_snapshot",
    "assert_json_snapshot",
    "assert_ron_snapshot",
    "assert_snapshot",
    "assert_toml_snapshot",
    "assert_yaml_snapshot",
];

/// One semantically resolved public Insta macro invocation and its source arguments.
///
/// The value retains both parsed top-level arguments and the complete source so a
/// caller can validate the invocation or produce an exact machine-applicable fix.
#[derive(Clone, Debug)]
pub struct InstaMacroInvocation {
    /// Span of the complete source-level macro call used for diagnostics.
    pub span: Span,
    /// Public macro name without the crate path or exclamation mark.
    pub name: String,
    /// Top-level source arguments supplied to the macro, preserving their order.
    pub arguments: Vec<String>,
    /// Complete source text for the macro invocation.
    source: String,
}

/// Recover one resolved public Insta macro invocation from expanded HIR.
///
/// Expansion metadata identifies the public macro, while source parsing retains
/// argument boundaries without evaluating arbitrary user code.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_names| {
///     let _ = insta_support::insta_macro_invocation(cx, expr, expected_names);
/// };
/// ```
pub fn insta_macro_invocation(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    expected_names: &[&str],
) -> Option<InstaMacroInvocation> {
    let (name, span) = insta_macro_expansion(cx, expr.span, expected_names)?;
    let source = cx.sess().source_map().span_to_snippet(span).ok()?;
    let arguments = macro_arguments(&source)?;
    Some(InstaMacroInvocation {
        span,
        name,
        arguments,
        source,
    })
}

/// Replace the public macro name while preserving its path and arguments.
///
/// The replacement succeeds only when the recorded source contains the expected
/// macro name immediately before the invocation bang, behind an `insta::` or
/// `::insta::` path. An imported or re-exported name may not cover the replacement.
#[must_use]
///
/// # Examples
///
/// ```rust
/// #![feature(rustc_private)]
/// use insta_support::{InstaMacroInvocation, insta_macro_replacement};
///
/// let replace_snapshot: fn(&InstaMacroInvocation) -> Option<String> = |invocation| {
///     insta_macro_replacement(invocation, "assert_snapshot")
/// };
/// let _ = replace_snapshot;
/// ```
pub fn insta_macro_replacement(
    invocation: &InstaMacroInvocation,
    replacement_name: &str,
) -> Option<String> {
    let name_range = macro_name_range(&invocation.source, &invocation.name)?;
    // Only an `insta::` path is sure to resolve the replacement macro as well.
    let path_prefix = invocation.source.get(..name_range.start)?.trim_start();
    if !matches!(path_prefix, "insta::" | "::insta::") {
        return None;
    }
    let mut replacement = invocation.source.clone();
    replacement.replace_range(name_range, replacement_name);
    Some(replacement)
}

/// Locate the exact macro-name range immediately before the invocation bang.
fn macro_name_range(source: &str, name: &str) -> Option<std::ops::Range<usize>> {
    let bang = source.find('!')?;
    let name_start = source.get(..bang)?.rfind(name)?;
    let name_end = name_start.checked_add(name.len())?;
    (source.get(name_end..=bang) == Some("!")).then_some(name_start..name_end)
}

/// Return whether the expression is nested in a source-level loop.
///
/// The invocation span excludes the loop introduced by a macro expansion itself,
/// so callers see only source-level loops written around the snapshot assertion.
/// The walk stops at the enclosing item, so a loop around a nested function
/// definition does not count.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, invocation_span| {
///     let _ = insta_support::is_in_loop(cx, expr, invocation_span);
/// };
/// ```
pub fn is_in_loop(cx: &LateContext<'_>, expr: &Expr<'_>, invocation_span: Span) -> bool {
    cx.tcx
        .hir_parent_iter(expr.hir_id)
        .take_while(|(_, node)| {
            !matches!(node, Node::Item(_) | Node::ImplItem(_) | Node::TraitItem(_))
        })
        .any(|(_, node)| {
            matches!(
                node,
                Node::Expr(parent)
                    if matches!(parent.kind, ExprKind::Loop(..))
                        && parent.span.source_callsite() != invocation_span
            )
        })
}

/// Return whether an Insta `allow_duplicates!` expansion encloses this expression.
///
/// The check finds the helper call that both `allow_duplicates!` and a direct
/// `with_allow_duplicates` call produce.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::is_in_allow_duplicates(cx, expr);
/// };
/// ```
pub fn is_in_allow_duplicates(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // `allow_duplicates!` expands to a call of this helper around its body.
    cx.tcx.hir_parent_iter(expr.hir_id).any(|(_, node)| {
        matches!(
            node,
            Node::Expr(Expr { kind: ExprKind::Call(callee, _), .. })
                if is_insta_function(cx, callee, "with_allow_duplicates")
        )
    })
}

/// Maximum number of local and constant bindings followed during extraction.
const STRING_LITERAL_RESOLUTION_LIMIT: usize = 8;

/// Extract a known string argument without evaluating arbitrary code.
///
/// The expression must be a string literal or resolve through at most eight simple
/// immutable local or current-crate constant bindings to a string literal. Mutable
/// bindings, destructuring, calls, and other computed expressions remain unknown.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::string_literal(cx, expr);
/// };
/// ```
pub fn string_literal(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<String> {
    string_literal_with_remaining(cx, expr, STRING_LITERAL_RESOLUTION_LIMIT)
}

/// Resolve a literal through a bounded chain of constants and immutable locals.
fn string_literal_with_remaining(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
    remaining: usize,
) -> Option<String> {
    if let ExprKind::Lit(literal) = expr.kind {
        return literal.node.str().map(|value| value.as_str().to_owned());
    }
    if remaining == 0 {
        return None;
    }

    let ExprKind::Path(ref path) = expr.kind else {
        return None;
    };
    match cx.qpath_res(path, expr.hir_id) {
        Res::Def(DefKind::Const { .. }, def_id) => {
            // Read only a current-crate constant body; external values stay unknown.
            let body = cx.tcx.hir_maybe_body_owned_by(def_id.as_local()?)?;
            string_literal_with_remaining(cx, body.value, remaining - 1)
        }
        Res::Local(binding) => {
            // Follow only a direct immutable local binding with a visible initializer.
            let Some((_, Node::LetStmt(local))) = cx.tcx.hir_parent_iter(binding).next() else {
                return None;
            };
            let PatKind::Binding(BindingMode::NONE, binding_id, _, None) = local.pat.kind else {
                return None;
            };
            if binding_id != binding || local.pat.hir_id != binding {
                return None;
            }
            string_literal_with_remaining(cx, local.init?, remaining - 1)
        }
        Res::Def(..)
        | Res::PrimTy(_)
        | Res::SelfTyParam { .. }
        | Res::SelfTyAlias { .. }
        | Res::SelfCtor(_)
        | Res::ToolMod
        | Res::OpenMod(_)
        | Res::NonMacroAttr(_)
        | Res::Err => None,
    }
}

/// Extract the source contents of one complete cooked or raw string literal.
///
/// The parser preserves literal contents and validates raw-string hash delimiters,
/// allowing callers to compare source text without rewriting escape sequences.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |source| {
///     let _ = insta_support::source_string_literal(source);
/// };
/// ```
pub fn source_string_literal(source: &str) -> Option<String> {
    // Trim surrounding source whitespace before classifying the literal form.
    let source = source.trim();
    // Extract ordinary strings without applying raw-string delimiter rules.
    if let Some(without_opening) = source.strip_prefix('"')
        && let Some(contents) = without_opening.strip_suffix('"')
    {
        return Some(contents.to_owned());
    }
    let prefix_end = source.find('"')?;
    // Require a raw prefix followed only by hash delimiters.
    let prefix = source.get(..prefix_end)?;
    let is_raw_prefix = prefix.starts_with('r');
    if !is_raw_prefix {
        return None;
    }
    let hashes = prefix.get(1..)?;
    let has_only_hashes = hashes.bytes().all(|byte| byte == b'#');
    if !has_only_hashes {
        return None;
    }
    // Remove the closing quote and the exact opening hash sequence.
    let suffix = format!("\"{hashes}");
    source
        .get(prefix_end + 1..)?
        .strip_suffix(&suffix)
        .map(str::to_owned)
}

/// Return whether an expression has Insta's public `Content` type.
///
/// The resolved nominal type must belong to Insta and be named `Content`,
/// excluding local structs that happen to use the same type name.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::is_content_expression(cx, expr);
/// };
/// ```
pub fn is_content_expression(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    is_insta_type(cx, cx.typeck_results().expr_ty(expr).peel_refs(), "Content")
}

/// Return whether a type is the named nominal type defined by Insta.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, ty, type_name| {
///     let _ = insta_support::is_insta_type(cx, ty, type_name);
/// };
/// ```
#[must_use]
pub fn is_insta_type(cx: &LateContext<'_>, ty: Ty<'_>, type_name: &str) -> bool {
    ty.ty_adt_def().is_some_and(|definition| {
        is_insta_item(cx, definition.did())
            && cx.tcx.item_name(definition.did()).as_str() == type_name
    })
}

/// Return whether a callee path resolves to the named function in the `insta` crate.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, callee, name| {
///     let _ = insta_support::is_insta_function(cx, callee, name);
/// };
/// ```
#[must_use]
pub fn is_insta_function(cx: &LateContext<'_>, callee: &Expr<'_>, name: &str) -> bool {
    matches!(
        callee.kind,
        ExprKind::Path(ref path)
            if matches!(
                cx.qpath_res(path, callee.hir_id),
                Res::Def(_, def_id)
                    if is_insta_item(cx, def_id) && cx.tcx.item_name(def_id).as_str() == name
            )
    )
}

/// Return whether a definition belongs to the `insta` crate.
fn is_insta_item(cx: &LateContext<'_>, def_id: DefId) -> bool {
    cx.tcx.crate_name(def_id.krate).as_str() == "insta"
}

/// Return whether a definition is the named inherent method of one Insta type.
fn is_insta_method(cx: &LateContext<'_>, def_id: DefId, owner: &str, method_name: &str) -> bool {
    // Check the method, then the self type of the impl block that defines it.
    if !is_insta_item(cx, def_id) || cx.tcx.item_name(def_id).as_str() != method_name {
        return false;
    }
    cx.tcx.opt_parent(def_id).is_some_and(|container| {
        matches!(cx.tcx.def_kind(container), DefKind::Impl { .. })
            && is_insta_type(
                cx,
                cx.tcx
                    .type_of(container)
                    .instantiate_identity()
                    .skip_norm_wip(),
                owner,
            )
    })
}

/// Return whether a resolved method belongs to Insta's `Content` type.
///
/// The helper returns the receiver only for the exact external owner and requested
/// method, allowing callers to inspect the value without repeating resolution.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method| {
///     let _ = insta_support::content_method_call(cx, expr, expected_method);
/// };
/// ```
pub fn content_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
) -> Option<&'hir Expr<'hir>> {
    // Resolve only method calls before checking the canonical Content owner.
    let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_insta_method(cx, def_id, "Content", expected_method).then_some(receiver)
}

/// One semantically resolved `insta::Settings` method call and its arguments.
///
/// The result retains the method span and borrowed argument slice needed by a
/// contextual settings lint without copying HIR expressions.
#[derive(Clone, Copy, Debug)]
pub struct SettingsMethodCall<'hir> {
    /// Span of the method identifier used for the primary diagnostic.
    pub method_span: Span,
    /// Explicit method arguments borrowed from the original HIR expression.
    pub arguments: &'hir [Expr<'hir>],
}

/// Resolve a method call to one exact `insta::Settings` definition.
///
/// Type-dependent lookup and owner matching exclude local methods that reuse an
/// Insta setting name without implementing the external Settings contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method| {
///     let _ = insta_support::settings_method_call(cx, expr, expected_method);
/// };
/// ```
pub fn settings_method_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
) -> Option<SettingsMethodCall<'hir>> {
    let ExprKind::MethodCall(segment, _, arguments, _) = expr.kind else {
        return None;
    };
    let def_id = cx.typeck_results().type_dependent_def_id(expr.hir_id)?;
    is_settings_def(cx, def_id, expected_method).then_some(SettingsMethodCall {
        method_span: segment.ident.span,
        arguments,
    })
}

/// Return whether a resolved call returns a value that implements `Future`.
///
/// Insta settings methods such as `Settings::bind` return their callable's
/// output type directly. Inspecting the inferred call result covers closures,
/// function items, and local callable bindings without relying on their syntax.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::call_returns_future(cx, expr);
/// };
/// ```
pub fn call_returns_future(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Resolve the standard Future trait before asking the type solver about this result.
    let Some(future_trait) = cx.tcx.lang_items().future_trait() else {
        return false;
    };
    // `Settings::bind` returns the closure output unchanged, so the call type is that output.
    let output = cx.typeck_results().expr_ty(expr);
    cx.tcx
        .infer_ctxt()
        .build(cx.typing_mode())
        .type_implements_trait(future_trait, [output], cx.param_env)
        .must_apply_modulo_regions()
}

/// Match a Settings setter whose only argument is an empty string.
///
/// The argument must be an empty string literal or a local constant initialized
/// with one, leaving computed values outside this contract.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method| {
///     let _ = insta_support::empty_string_settings_call(cx, expr, expected_method);
/// };
/// ```
pub fn empty_string_settings_call<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
) -> Option<SettingsMethodCall<'hir>> {
    let call = settings_method_call(cx, expr, expected_method)?;
    matches!(call.arguments, [argument] if string_literal(cx, argument).is_some_and(|value| value.is_empty()))
        .then_some(call)
}

/// A contextual condition for a Settings method.
///
/// Conditions form a closed set shared by the generated lint declarations, making
/// each diagnostic's accepted argument and execution context explicit.
#[derive(Clone, Copy, Debug)]
pub enum SettingsMethodCondition {
    /// Every resolved call is discouraged by the selected Settings lint rule.
    Any,
    /// The single argument is the literal `false`, disabling the setting.
    FalseArgument,
    /// The single argument is the literal `true`, enabling the setting.
    TrueArgument,
    /// The call executes in an async body where binding has special behavior.
    InAsyncBody,
}

/// Match a Settings method under one documented contextual condition.
///
/// The helper combines exact method resolution with the selected argument or
/// async-context predicate, returning the original method span for diagnostics.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr, expected_method, condition| {
///     let _ = insta_support::settings_method_violation(cx, expr, expected_method, condition);
/// };
/// ```
pub fn settings_method_violation<'hir>(
    cx: &LateContext<'_>,
    expr: &'hir Expr<'hir>,
    expected_method: &str,
    condition: SettingsMethodCondition,
) -> Option<SettingsMethodCall<'hir>> {
    let call = settings_method_call(cx, expr, expected_method)?;
    let is_match = match condition {
        SettingsMethodCondition::Any => true,
        SettingsMethodCondition::FalseArgument => matches!(
            call.arguments,
            [argument]
                if matches!(
                    argument.kind,
                    ExprKind::Lit(literal)
                        if matches!(literal.node, LitKind::Bool(false))
                )
        ),
        SettingsMethodCondition::TrueArgument => matches!(
            call.arguments,
            [argument]
                if matches!(
                    argument.kind,
                    ExprKind::Lit(literal)
                        if matches!(literal.node, LitKind::Bool(true))
                )
        ),
        SettingsMethodCondition::InAsyncBody => is_in_async_body(cx, expr),
    };
    is_match.then_some(call)
}

/// Return whether the nearest closure boundary is async.
///
/// A synchronous nested closure intentionally breaks inheritance from its outer
/// async function, matching the execution context observed by the settings call.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::is_in_async_body(cx, expr);
/// };
/// ```
pub fn is_in_async_body(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // The nearest closure boundary decides the expression's async context.
    for (_, node) in cx.tcx.hir_parent_iter(expr.hir_id) {
        let Node::Expr(parent) = node else { continue };
        let ExprKind::Closure(closure) = parent.kind else {
            continue;
        };
        // Stop at this boundary so nested blocking closures remain synchronous.
        return is_async_closure_kind(closure.kind);
    }
    false
}

/// A call that creates `insta::Settings` from Insta's defaults.
#[derive(Clone, Copy, Debug)]
pub struct SettingsDefaultsCall {
    /// Span of the constructor name to replace with `clone_current`, when the
    /// call is spelled `<type>::new()` or `<type>::default()`.
    pub name_span: Option<Span>,
}

/// Match `Settings::new()`, `Settings::default()`, or `Default::default()`
/// producing `insta::Settings`.
///
/// The constructor name can be replaced only when the call names the type, as in
/// `Settings::new()`. A trait path such as `Default::default()` gets no name span.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::settings_defaults_call(cx, expr);
/// };
/// ```
pub fn settings_defaults_call(
    cx: &LateContext<'_>,
    expr: &Expr<'_>,
) -> Option<SettingsDefaultsCall> {
    // Require a direct zero-argument path call that produces Insta's Settings.
    let ExprKind::Call(callee, []) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let def_id = cx.qpath_res(path, callee.hir_id).opt_def_id()?;
    if !is_insta_type(cx, cx.typeck_results().expr_ty(expr), "Settings") {
        return None;
    }

    // Accept Insta's constructor or any resolution of `Default::default`.
    let is_new = is_insta_method(cx, def_id, "Settings", "new");
    let is_default = cx.tcx.item_name(def_id).as_str() == "default"
        && (is_insta_item(cx, def_id)
            || cx.tcx.opt_parent(def_id) == cx.tcx.get_diagnostic_item(sym::Default));
    if !is_new && !is_default {
        return None;
    }
    let name_span = if let QPath::TypeRelative(_, segment) = path {
        (!segment.ident.span.from_expansion()).then_some(segment.ident.span)
    } else {
        None
    };
    Some(SettingsDefaultsCall { name_span })
}

/// Declare one contextual Settings method lint.
#[macro_export]
macro_rules! declare_settings_method_lint {
    (
        $lint:ident, $pass:ident, $method:literal, $condition:ident,
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
            /// Check one resolved Settings method under its contextual condition.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(call) = $crate::settings_method_violation(
                    cx,
                    expr,
                    $method,
                    $crate::SettingsMethodCondition::$condition,
                ) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.method_span,
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

/// Declare one empty-string Insta Settings lint.
#[macro_export]
macro_rules! declare_empty_settings_lint {
    ($lint:ident, $pass:ident, $method:literal, $description:literal, $help:literal) => {
        dylint_support::documented_late_lint! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass
        }
        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check the resolved Settings setter and literal argument.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(call) = $crate::empty_string_settings_call(cx, expr, $method) else {
                    return;
                };
                cx.emit_span_lint(
                    $lint,
                    call.method_span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message("this Insta setting is an empty string")
                            .help($help);
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

/// Declare one lint for a discouraged public Insta macro.
#[macro_export]
macro_rules! declare_discouraged_macro_lint {
    (
        $lint:ident, $pass:ident, $macro_name:literal,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint_with_pass! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass,
            $pass::default()
        }

        /// Track source macro invocations already visited through expanded HIR.
        #[derive(Debug, Default)]
        pub struct $pass {
            /// Complete invocation spans that already emitted a diagnostic.
            reported: std::collections::HashSet<rustc_span::Span>,
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check each resolved macro invocation once.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(invocation) = $crate::insta_macro_invocation(cx, expr, &[$macro_name])
                else {
                    return;
                };
                if !self.reported.insert(invocation.span) {
                    return;
                }
                cx.emit_span_lint(
                    $lint,
                    invocation.span,
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

/// Declare a discouraged Insta macro lint with an exact macro-name replacement.
#[macro_export]
macro_rules! declare_discouraged_macro_replacement_lint {
    (
        $lint:ident, $pass:ident, $macro_name:literal, $replacement_name:literal,
        $description:literal, $message:literal, $help:literal
    ) => {
        dylint_support::documented_late_lint_with_pass! {
            #[doc = include_str!("../README.md")]
            pub $lint,
            Warn,
            $description,
            $pass,
            $pass::default()
        }

        /// Track source macro invocations already visited through expanded HIR.
        #[derive(Debug, Default)]
        pub struct $pass {
            /// Complete invocation spans that already emitted a diagnostic.
            reported: std::collections::HashSet<rustc_span::Span>,
        }

        impl<'tcx> rustc_lint::LateLintPass<'tcx> for $pass {
            /// Check each resolved macro invocation once.
            fn check_expr(
                &mut self,
                cx: &rustc_lint::LateContext<'tcx>,
                expr: &'tcx rustc_hir::Expr<'tcx>,
            ) {
                let Some(invocation) = $crate::insta_macro_invocation(cx, expr, &[$macro_name])
                else {
                    return;
                };
                if !self.reported.insert(invocation.span) {
                    return;
                }
                let replacement = $crate::insta_macro_replacement(&invocation, $replacement_name);
                cx.emit_span_lint(
                    $lint,
                    invocation.span,
                    rustc_errors::DiagDecorator(|diagnostic| {
                        let diagnostic = diagnostic.primary_message($message);
                        if let Some(replacement) = replacement {
                            let _configured_suggestion = diagnostic.span_suggestion(
                                invocation.span,
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

        #[test]
        fn ui() {
            dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
        }
    };
}

/// Return whether a closure is explicitly async or immediately returns an async block.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |cx, expr| {
///     let _ = insta_support::is_explicit_async_closure(cx, expr);
/// };
/// ```
pub fn is_explicit_async_closure(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Reject non-closure expressions before reading coroutine metadata.
    let ExprKind::Closure(closure) = expr.kind else {
        return false;
    };
    if is_async_closure_kind(closure.kind) {
        return true;
    }

    // A synchronous `|| async { ... }` closure returns after `Settings::bind` has reset.
    let body = cx.tcx.hir_body(closure.body);
    is_async_expression(body.value)
}

/// Return whether a definition is the requested `insta::Settings` method.
fn is_settings_def(cx: &LateContext<'_>, def_id: DefId, expected_method: &str) -> bool {
    is_insta_method(cx, def_id, "Settings", expected_method)
}

/// Recognize both async blocks and async closures.
const fn is_async_closure_kind(kind: ClosureKind) -> bool {
    matches!(
        kind,
        ClosureKind::Coroutine(CoroutineKind::Desugared(CoroutineDesugaring::Async, _))
            | ClosureKind::CoroutineClosure(CoroutineDesugaring::Async)
    )
}

/// Strip compiler wrappers before checking for an async block expression.
fn is_async_expression(expr: &Expr<'_>) -> bool {
    // Recognize closure nodes before peeling compiler-generated wrappers.
    if let ExprKind::Closure(closure) = expr.kind {
        return is_async_closure_kind(closure.kind);
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

/// Walk expansion hygiene until a public Insta macro is found.
fn insta_macro_expansion(
    cx: &LateContext<'_>,
    span: Span,
    expected_names: &[&str],
) -> Option<(String, Span)> {
    // Walk outward from expanded HIR toward the outermost public Insta invocation,
    // because Insta macros call themselves recursively from inside their expansion.
    let mut outermost = None;
    let mut context = span.ctxt();
    while context != SyntaxContext::root() {
        // Accept only requested macro names whose resolved definition belongs to Insta.
        let expansion = context.outer_expn_data();
        if let Some(public_name) = insta_public_macro_name(cx, &expansion, expected_names) {
            outermost = Some((public_name, expansion.call_site));
        }

        // Stop when malformed hygiene points back to the same context.
        let next = expansion.call_site.ctxt();
        context = if next == context {
            SyntaxContext::root()
        } else {
            next
        };
    }
    outermost
}

/// Return the public name of a requested bang macro defined by Insta.
fn insta_public_macro_name(
    cx: &LateContext<'_>,
    expansion: &ExpnData,
    expected_names: &[&str],
) -> Option<String> {
    // Only bang macros with a resolved definition can be public Insta assertions.
    let (ExpnKind::Macro(MacroKind::Bang, macro_name), Some(def_id)) =
        (&expansion.kind, expansion.macro_def_id)
    else {
        return None;
    };
    let public_name = macro_name.as_str().rsplit("::").next()?;
    // Reject same-named macros from other crates and names outside the request.
    let is_insta_macro = cx.tcx.crate_name(def_id.krate).as_str() == "insta";
    let is_expected_name = expected_names.contains(&public_name);
    (is_insta_macro && is_expected_name).then(|| public_name.to_owned())
}

/// Split a complete macro invocation into top-level comma-separated arguments.
fn macro_arguments(source: &str) -> Option<Vec<String>> {
    // Determine the invocation delimiter from source immediately after the bang.
    let after_bang = source.get(source.find('!')? + 1..)?.trim_start();
    let opening = after_bang.as_bytes().first().copied()?;
    let closing = match opening {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        _ => return None,
    };
    let end = matching_delimiter(after_bang, opening, closing)?;
    // Split top-level commas and discard empty argument slots.
    let contents = after_bang.get(1..end)?;
    Some(
        split_top_level(contents)
            .into_iter()
            .map(str::trim)
            .filter(|argument| !argument.is_empty())
            .map(str::to_owned)
            .collect(),
    )
}

/// Find the closing delimiter of the complete macro invocation.
fn matching_delimiter(source: &str, opening: u8, closing: u8) -> Option<usize> {
    // Track nested copies of the invocation delimiter pair.
    let bytes = source.as_bytes();
    let mut index = 0_usize;
    let mut depth = 0_usize;
    while index < bytes.len() {
        // Ignore delimiter bytes inside literals and comments.
        if let Some(next) = skipped_token_end(source, index) {
            index = next;
            continue;
        }
        match *bytes.get(index)? {
            byte if byte == opening => depth = depth.checked_add(1)?,
            byte if byte == closing => {
                // The first return to zero closes the outer invocation.
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Split source on top-level commas while preserving nested Rust syntax.
fn split_top_level(source: &str) -> Vec<&str> {
    // Preserve borrowed source slices and their original order.
    let bytes = source.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0_usize;
    let mut index = 0_usize;
    // Store expected closers for every nested Rust group.
    let mut delimiters = Vec::new();
    while let Some(&byte) = bytes.get(index) {
        // Skip literals and comments because their commas are data.
        if let Some(next) = skipped_token_end(source, index) {
            index = next;
            continue;
        }
        // Split only on commas outside all nested groups.
        match byte {
            b'(' => delimiters.push(b')'),
            b'[' => delimiters.push(b']'),
            b'{' => delimiters.push(b'}'),
            byte if delimiters.last().copied() == Some(byte) => {
                let _closing = delimiters.pop();
            }
            b',' if delimiters.is_empty() => {
                if let Some(part) = source.get(start..index) {
                    parts.push(part);
                }
                start = index + 1;
            }
            _ => {}
        }
        index += 1;
    }
    // Append the segment after the final top-level comma.
    if let Some(part) = source.get(start..) {
        parts.push(part);
    }
    parts
}

/// Skip a literal or comment that starts at `index`.
fn skipped_token_end(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    match bytes.get(index..)? {
        [b'/', b'/', ..] => Some(
            source
                .get(index..)?
                .find('\n')
                .map_or(bytes.len(), |offset| index + offset + 1),
        ),
        [b'/', b'*', ..] => block_comment_end(bytes, index),
        [b'"', ..] => Some(cooked_literal_end(bytes, index, b'"')),
        [b'\'', ..] => char_literal_end(bytes, index),
        [b'b' | b'c', b'"', ..] => Some(cooked_literal_end(bytes, index + 1, b'"')),
        [b'b', b'\'', ..] => Some(cooked_literal_end(bytes, index + 1, b'\'')),
        [b'r', ..] => raw_literal_end(bytes, index),
        [b'b' | b'c', b'r', ..] => raw_literal_end(bytes, index + 1),
        _ => None,
    }
}

/// Skip one possibly nested block comment.
fn block_comment_end(bytes: &[u8], start: usize) -> Option<usize> {
    // Begin after the opening marker with one active comment level.
    let mut index = start + 2;
    let mut depth = 1_usize;
    while index + 1 < bytes.len() {
        // Count nested openers and closers without parsing comment text.
        match bytes.get(index..index + 2)? {
            [b'/', b'*'] => {
                depth = depth.checked_add(1)?;
                index += 2;
            }
            [b'*', b'/'] => {
                depth = depth.checked_sub(1)?;
                index += 2;
                if depth == 0 {
                    // Return the first byte after the balanced closing marker.
                    return Some(index);
                }
            }
            _ => index += 1,
        }
    }
    Some(bytes.len())
}

/// Skip a cooked string, byte string, character, or byte literal.
fn cooked_literal_end(bytes: &[u8], quote: usize, delimiter: u8) -> usize {
    // Start after the opening quote and skip escaped byte pairs together.
    let mut index = quote + 1;
    while let Some(&byte) = bytes.get(index) {
        // Return after the first unescaped matching delimiter.
        match byte {
            b'\\' => index = (index + 2).min(bytes.len()),
            byte if byte == delimiter => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// Treat a quote as a character literal only when its closing quote is nearby.
fn char_literal_end(bytes: &[u8], start: usize) -> Option<usize> {
    let end = cooked_literal_end(bytes, start, b'\'');
    (end > start + 2 && end <= start + 8).then_some(end)
}

/// Skip a raw string after its `r` prefix.
fn raw_literal_end(bytes: &[u8], raw_prefix: usize) -> Option<usize> {
    // Count opening hashes before validating the raw string quote.
    let mut quote = raw_prefix + 1;
    while bytes.get(quote) == Some(&b'#') {
        quote += 1;
    }
    if bytes.get(quote) != Some(&b'"') {
        return None;
    }
    // Preserve the exact opening hash sequence for closing validation.
    let hashes = quote.checked_sub(raw_prefix + 1)?;
    let expected_hashes = bytes.get(raw_prefix + 1..quote)?;
    let mut index = quote + 1;
    while index < bytes.len() {
        // Accept a quote only when every required closing hash follows it.
        if is_raw_string_close(bytes, index, expected_hashes) {
            return Some(index + 1 + hashes);
        }
        index += 1;
    }
    // Treat unterminated raw source as extending through the available bytes.
    Some(bytes.len())
}

/// Return whether a quote at `index` is followed by the expected closing hashes.
fn is_raw_string_close(bytes: &[u8], index: usize, expected_hashes: &[u8]) -> bool {
    let is_quote = bytes.get(index) == Some(&b'"');
    let hashes_start = index + 1;
    let hashes_end = hashes_start + expected_hashes.len();
    is_quote && bytes.get(hashes_start..hashes_end) == Some(expected_hashes)
}

/// Public assertion macro names used by loop-related lints.
///
/// The returned static slice is shared by loop checks so every lint uses the same
/// closed list of public snapshot assertion macros.
#[must_use]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = || {
///     let _ = insta_support::snapshot_macro_names();
/// };
/// ```
pub const fn snapshot_macro_names() -> &'static [&'static str] {
    SNAPSHOT_MACROS
}

#[cfg(test)]
mod tests {
    use super::{
        InstaMacroInvocation, insta_macro_replacement, macro_arguments, source_string_literal,
    };
    use rustc_span::DUMMY_SP;

    #[test]
    fn macro_replacement_preserves_path_and_arguments() {
        let invocation = InstaMacroInvocation {
            span: DUMMY_SP,
            name: "assert_display_snapshot".to_owned(),
            arguments: vec!["value".to_owned()],
            source: "insta::assert_display_snapshot!(value)".to_owned(),
        };

        assert_eq!(
            insta_macro_replacement(&invocation, "assert_snapshot"),
            Some("insta::assert_snapshot!(value)".to_owned())
        );
    }

    #[test]
    fn macro_replacement_rejects_non_macro_source_shape() {
        let invocation = InstaMacroInvocation {
            span: DUMMY_SP,
            name: "assert_display_snapshot".to_owned(),
            arguments: Vec::new(),
            source: "assert_display_snapshot (value)".to_owned(),
        };

        assert_eq!(
            insta_macro_replacement(&invocation, "assert_snapshot"),
            None
        );
    }

    #[test]
    fn macro_arguments_skip_literals_and_comments() {
        let source = concat!(
            "insta::assert_snapshot!(\n",
            "    \"a, (b\", // c, )\n",
            "    /* d, /* e, */ f) */ 'g',\n",
            "    b\"h, )\", br#\"i, \"j\"#, ',', b',',\n",
            "    [k, (l, m)], {n, o}, r\"p, q\", \"s\\\", t\",\n",
            ")",
        );

        assert_eq!(
            macro_arguments(source),
            Some(
                [
                    "\"a, (b\"",
                    concat!("/", "/ c, )\n    /* d, /* e, */ f) */ 'g'"),
                    "b\"h, )\"",
                    "br#\"i, \"j\"#",
                    "','",
                    "b','",
                    "[k, (l, m)]",
                    "{n, o}",
                    "r\"p, q\"",
                    "\"s\\\", t\"",
                ]
                .map(str::to_owned)
                .to_vec()
            )
        );
    }

    #[test]
    fn macro_arguments_accept_each_delimiter_and_reject_others() {
        let actual = [
            macro_arguments("insta::glob![a, b]"),
            macro_arguments("insta::glob!{a}"),
            macro_arguments("insta::glob!<a>"),
            macro_arguments("insta::glob!(a"),
            macro_arguments("no_bang(a)"),
        ];
        let expected = [
            Some(vec!["a".to_owned(), "b".to_owned()]),
            Some(vec!["a".to_owned()]),
            None,
            None,
            None,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn macro_arguments_tolerate_unterminated_source() {
        let actual = [
            macro_arguments("m!(a /* open"),
            macro_arguments("m!(r#\"open"),
            macro_arguments("m!(\"open"),
            macro_arguments("m!(r, 'lifetime)"),
        ];
        let expected = [
            None,
            None,
            None,
            Some(vec!["r".to_owned(), "'lifetime".to_owned()]),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn source_string_literal_reads_cooked_and_raw_forms() {
        let actual = [
            source_string_literal(" \"../a\" "),
            source_string_literal("r\"../a\""),
            source_string_literal("r##\"../a\"##"),
            source_string_literal("r#\"../a\"##"),
            source_string_literal("b\"../a\""),
            source_string_literal("rx\"../a\""),
            source_string_literal("value"),
            source_string_literal("r##\"../a\"#"),
        ];
        let expected = [
            Some("../a".to_owned()),
            Some("../a".to_owned()),
            Some("../a".to_owned()),
            None,
            None,
            None,
            None,
            None,
        ];
        assert_eq!(actual, expected);
    }
}
