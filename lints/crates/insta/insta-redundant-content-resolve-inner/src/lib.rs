#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `Content::resolve_inner` immediately followed by an `as_*` accessor.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use insta_support::content_method_call;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_REDUNDANT_CONTENT_RESOLVE_INNER,
    Warn,
    "Content::resolve_inner is redundantly called before an Insta accessor",
    InstaRedundantContentResolveInner
}

impl<'tcx> LateLintPass<'tcx> for InstaRedundantContentResolveInner {
    /// Check resolved `Content::as_*` calls for a resolved inner-call receiver.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the complete replacement before emitting one source diagnostic.
        let Some((receiver_span, replacement)) = redundant_receiver_replacement(cx, expr) else {
            return;
        };

        cx.emit_span_lint(
            INSTA_REDUNDANT_CONTENT_RESOLVE_INNER,
            receiver_span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("Insta `Content::as_*` accessors already resolve wrappers")
                    .span_suggestion(
                        receiver_span,
                        "call the accessor directly on the original `Content` value",
                        replacement,
                        Applicability::MachineApplicable,
                    );
            }),
        );
    }
}

/// Resolve one accessor whose receiver is an unnecessary inner-resolution call.
fn redundant_receiver_replacement(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<(Span, String)> {
    // Resolve the outer accessor before inspecting its receiver expression.
    let ExprKind::MethodCall(segment, receiver, _, _) = expr.kind else {
        return None;
    };
    let method_name = segment.ident.name.as_str();
    if !method_name.starts_with("as_") || content_method_call(cx, expr, method_name).is_none() {
        return None;
    }

    // Report only a resolved inner call immediately below the accessor.
    let original_receiver = content_method_call(cx, receiver, "resolve_inner")?;
    let replacement = cx
        .sess()
        .source_map()
        .span_to_snippet(original_receiver.span)
        .ok()?;
    Some((receiver.span, replacement))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
