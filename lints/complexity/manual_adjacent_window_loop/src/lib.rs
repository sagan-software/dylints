#![feature(rustc_private)]
#![warn(unused_extern_crates)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "the lint intentionally ignores unmatched rustc syntax variants"
)]

//! A lint for index loops that can use `slice::windows`.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_middle;
extern crate rustc_span;

#[path = "../../chaining_lint_support.rs"]
mod support;

use rustc_hir::{
    Expr, ExprKind,
    intravisit::{Visitor, walk_expr},
};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ADJACENT_WINDOW_LOOP,
    Warn,
    "adjacent index loop can use `slice::windows(2)`",
    ManualAdjacentWindowLoop
}

impl<'tcx> LateLintPass<'tcx> for ManualAdjacentWindowLoop {
    /// Checks a standard loop for the non-panicking adjacent-pair range.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(span) = adjacent_loop_span(cx, expr) {
            support::emit(
                cx,
                MANUAL_ADJACENT_WINDOW_LOOP,
                span,
                "adjacent index loop can use `slice::windows(2)`",
                "use `windows(2)` and read the two elements from each window",
            );
        }
    }
}

/// Return the span of a loop with the exact adjacent-index shape.
fn adjacent_loop_span<'tcx>(cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) -> Option<Span> {
    // Recover the desugared loop before checking its source and body independently.
    (!matches!(expr.kind, ExprKind::DropTemps(_)))
        .then_some(expr)
        .and_then(|expr| support::for_loop(cx, expr))
        .and_then(|loop_info| {
            support::snippet(cx, loop_info.source.span).map(|source| (loop_info, source))
        })
        .filter(|(_, source)| is_adjacent_range(source))
        .and_then(|(loop_info, _)| {
            let mut source_types = SliceUseVisitor {
                cx,
                slice_len_calls: 0,
                slice_indexes: 0,
            };
            // Count semantic slice operations across both the range and loop body.
            source_types.visit_expr(loop_info.source);
            source_types.visit_block(loop_info.body);
            support::snippet(cx, loop_info.body.span)
                .map(|body_source| (loop_info, source_types, body_source))
        })
        .filter(|(_, source_types, body_source)| {
            source_types.slice_len_calls == 1
                && source_types.slice_indexes == 2
                && body_source.contains(" + 1]")
        })
        .map(|(loop_info, _, _)| loop_info.span)
}

/// Return whether a source range is the supported zero-based adjacent form.
fn is_adjacent_range(source: &str) -> bool {
    source.trim_start().starts_with("0..") && source.contains(".len().saturating_sub(1)")
}

/// Counts semantic slice length calls and slice indexing operations.
struct SliceUseVisitor<'cx, 'tcx> {
    /// Compiler context used for receiver types.
    cx: &'cx LateContext<'tcx>,
    /// Number of supported slice length calls.
    slice_len_calls: usize,
    /// Number of supported slice indexing operations.
    slice_indexes: usize,
}

impl<'tcx> Visitor<'tcx> for SliceUseVisitor<'_, 'tcx> {
    /// Visits expressions and records supported slice operations.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Count only operations whose receiver has a supported slice-like type.
        match expr.kind {
            ExprKind::MethodCall(segment, receiver, [], _)
                if segment.ident.name.as_str() == "len"
                    && support::is_slice_like(
                        self.cx,
                        self.cx.typeck_results().expr_ty(receiver),
                    ) =>
            {
                self.slice_len_calls += 1;
            }
            ExprKind::Index(receiver, _, _)
                if support::is_slice_like(self.cx, self.cx.typeck_results().expr_ty(receiver)) =>
            {
                self.slice_indexes += 1;
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

/// Runs the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
