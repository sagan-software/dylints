#![feature(rustc_private)]

//! A lint to check for nonportable Serde `deserialize_any` calls.
//!
//! This Dylint library resolves method and path calls to
//! `serde::Deserializer::deserialize_any` and reports them outside
//! `Deserializer` implementations, where they tie a type to self-describing
//! formats. Calls produced by macros are skipped, so forwarding macros such
//! as `forward_to_deserialize_any!` do not report.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use serde as _;

use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, def::Res};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

use serde_support::{is_in_serde_trait_impl, is_serde_trait_method, serde_method_call};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub SERDE_DESERIALIZE_ANY_PORTABILITY,
    Warn,
    "serde deserialize_any limits deserialization to self-describing formats",
    SerdeDeserializeAnyPortability
}

impl<'tcx> LateLintPass<'tcx> for SerdeDeserializeAnyPortability {
    /// Check semantically resolved `deserialize_any` calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(span) = deserialize_any_call(cx, expr) else {
            return;
        };
        // A format's own `Deserializer` forwards to `deserialize_any` by design.
        if span.from_expansion() || is_in_serde_trait_impl(cx, expr.hir_id, "Deserializer") {
            return;
        }

        cx.emit_span_lint(
            SERDE_DESERIALIZE_ANY_PORTABILITY,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message(
                        "`deserialize_any` restricts this code to self-describing formats",
                    )
                    .help("use a typed `deserialize_*` method unless dynamic input is intentional");
            }),
        );
    }
}

/// Return the callee span of a method or path call to `Deserializer::deserialize_any`.
fn deserialize_any_call(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Resolve `deserializer.deserialize_any(visitor)` through type-dependent lookup.
    if let Some(method) = serde_method_call(cx, expr, "Deserializer", "deserialize_any") {
        return Some(method.method_span);
    }
    // Resolve `Deserializer::deserialize_any(deserializer, visitor)` through its path.
    let ExprKind::Call(callee, _) = expr.kind else {
        return None;
    };
    let ExprKind::Path(ref path) = callee.kind else {
        return None;
    };
    let Res::Def(_, def_id) = cx.qpath_res(path, callee.hir_id) else {
        return None;
    };
    is_serde_trait_method(cx, def_id, "Deserializer", "deserialize_any").then_some(callee.span)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
