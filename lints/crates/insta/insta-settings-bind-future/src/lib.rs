#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for futures returned from synchronous Insta settings bindings.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Insta settings bindings, reports futures held
//! across synchronous calls, and recommends awaiting them before binding.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use insta_support::{is_explicit_async_closure, settings_method_call};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::{
    CaptureBy, ClosureKind, CoroutineDesugaring, CoroutineKind, CoroutineSource, Expr, ExprKind,
};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SETTINGS_BIND_FUTURE,
    Warn,
    "a future is returned from a synchronous Insta settings binding",
    InstaSettingsBindFuture
}

impl<'tcx> LateLintPass<'tcx> for InstaSettingsBindFuture {
    /// Check explicit futures returned from semantically resolved `Settings::bind` calls.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the exact bind method and require its sole closure argument.
        let Some(call) = settings_method_call(cx, expr, "bind") else {
            return;
        };
        let [closure] = call.arguments else {
            return;
        };
        // Report only closures whose result is an explicit future.
        if !is_explicit_async_closure(cx, closure) {
            return;
        }

        let closure_prefix = closure_prefix_span(cx, closure);

        cx.emit_span_lint(
            INSTA_SETTINGS_BIND_FUTURE,
            call.method_span,
            DiagDecorator(|diagnostic| {
                let help = "pass the async block directly to `Settings::bind_async` instead";
                let diagnostic = diagnostic
                    .primary_message("these settings reset before the returned future is polled");
                // Rename the method and drop the closure head so the async block becomes the argument.
                if let Some(closure_prefix) = closure_prefix {
                    let _configured_suggestion = diagnostic.multipart_suggestion(
                        help,
                        vec![
                            (call.method_span, "bind_async".to_owned()),
                            (closure_prefix, String::new()),
                        ],
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help = diagnostic.help(help);
                }
            }),
        );
    }
}

/// Return the `||` or `move ||` source before an async block returned by a closure.
///
/// Deleting that prefix turns `|| async { .. }` into the future that
/// `Settings::bind_async` takes. The closure must take no parameters and must
/// return the async block directly. A `move` closure must wrap an `async move`
/// block, because otherwise the block would borrow what the closure moved.
fn closure_prefix_span<'tcx>(
    cx: &LateContext<'tcx>,
    closure_expr: &'tcx Expr<'tcx>,
) -> Option<Span> {
    // Accept only a synchronous closure; async closures have no separate block to keep.
    let ExprKind::Closure(closure) = closure_expr.kind else {
        return None;
    };
    if !matches!(closure.kind, ClosureKind::Closure) {
        return None;
    }
    // Require the body to be a parameterless async block.
    let (body_value, async_capture) = direct_async_block(cx, closure)?;
    // Preserve captures when deleting the outer closure.
    let is_capture_preserving = matches!(closure.capture_clause, CaptureBy::Ref)
        || matches!(async_capture, CaptureBy::Value { .. });
    // Edit only user-written source that encloses the async block.
    is_capture_preserving
        .then(|| {
            let closure_span = closure_expr.span;
            let block_span = body_value.span;
            // Keep a fix only when both spans are written by the caller.
            (!closure_span.from_expansion()
                && !block_span.from_expansion()
                && closure_span.contains(block_span))
            .then(|| closure_span.until(block_span))
        })
        .flatten()
}

/// Return a direct async block and its capture mode.
fn direct_async_block<'tcx>(
    cx: &LateContext<'tcx>,
    closure: &'tcx rustc_hir::Closure<'tcx>,
) -> Option<(&'tcx Expr<'tcx>, CaptureBy)> {
    // Inspect the closure body and reject parameters before examining its value.
    let body = cx.tcx.hir_body(closure.body);
    if !body.params.is_empty() {
        return None;
    }
    let mut body_value = body.value;
    // Remove compiler-inserted temporary expressions around the returned block.
    while let ExprKind::DropTemps(inner) = body_value.kind {
        body_value = inner;
    }
    let ExprKind::Closure(async_block) = body_value.kind else {
        return None;
    };
    matches!(
        async_block.kind,
        ClosureKind::Coroutine(CoroutineKind::Desugared(
            CoroutineDesugaring::Async,
            CoroutineSource::Block
        ))
    )
    .then_some((body_value, async_block.capture_clause))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
