#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks snapshot assertions repeated by source-level loops.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use std::collections::HashSet;

use insta_support::{
    insta_macro_invocation, is_in_allow_duplicates, is_in_loop, snapshot_macro_names,
};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SNAPSHOT_IN_LOOP,
    Warn,
    "an Insta snapshot assertion is repeated in a loop without allow_duplicates",
    InstaSnapshotInLoop,
    InstaSnapshotInLoop::default()
}

/// Track source macro invocations already visited through expanded HIR.
/// The pass retains invocation spans so one macro expansion cannot emit
/// duplicate diagnostics for each generated expression it visits.
#[derive(Debug, Default)]
pub struct InstaSnapshotInLoop {
    /// Complete invocation spans that already emitted a diagnostic.
    reported: HashSet<Span>,
}

impl<'tcx> LateLintPass<'tcx> for InstaSnapshotInLoop {
    /// Check resolved snapshot macros inside direct source-level loops.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve a supported snapshot macro before checking its source-level context.
        let Some(invocation) = insta_macro_invocation(cx, expr, snapshot_macro_names()) else {
            return;
        };
        if !is_in_loop(cx, expr, invocation.span) {
            return;
        }
        if is_in_allow_duplicates(cx, expr) {
            return;
        }
        // Deduplicate expanded expressions that belong to one macro invocation.
        if !self.reported.insert(invocation.span) {
            return;
        }

        cx.emit_span_lint(
            INSTA_SNAPSHOT_IN_LOOP,
            invocation.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this assertion can visit the same snapshot more than once")
                    .help(
                        "use distinct snapshot names or wrap equal repeats in `allow_duplicates!`",
                    );
            }),
        );
    }
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
