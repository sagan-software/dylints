#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks parent traversal in Insta's two-argument `glob!` form.
//!
//! This Dylint library resolves Insta glob invocations, reports unsupported
//! parent traversal, and recommends the three-argument form with an explicit base.
//!
//! The README defines the supported macro shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use std::collections::HashSet;

use insta_support::{InstaMacroInvocation, insta_macro_invocation, source_string_literal};
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub INSTA_GLOB_PARENT_TRAVERSAL,
    Warn,
    "a two-argument Insta glob uses unsupported parent traversal",
    InstaGlobParentTraversal,
    InstaGlobParentTraversal::default()
}

/// Track source macro invocations already visited through expanded HIR.
///
/// The pass stores only deduplication state; invocation arguments remain in the
/// resolved support value until the diagnostic has been emitted.
#[derive(Debug, Default)]
pub struct InstaGlobParentTraversal {
    /// Complete invocation spans that already emitted a diagnostic.
    reported: HashSet<Span>,
}

impl<'tcx> LateLintPass<'tcx> for InstaGlobParentTraversal {
    /// Check resolved two-argument `insta::glob!` calls once.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve only two-argument Insta glob invocations with parent traversal.
        let Some(invocation) =
            insta_macro_invocation(cx, expr, &["glob"]).filter(has_glob_parent_pattern)
        else {
            return;
        };

        // Expanded HIR can visit one macro invocation more than once.
        if !self.reported.insert(invocation.span) {
            return;
        }

        cx.emit_span_lint(
            INSTA_GLOB_PARENT_TRAVERSAL,
            invocation.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("the two-argument `glob!` form cannot traverse parents")
                    .help("pass the parent directory as the base in the three-argument form");
            }),
        );
    }
}

/// Return whether a two-argument glob invocation contains literal parent traversal.
fn has_glob_parent_pattern(invocation: &InstaMacroInvocation) -> bool {
    let [pattern, _closure] = invocation.arguments.as_slice() else {
        return false;
    };
    source_string_literal(pattern)
        .is_some_and(|pattern| pattern == ".." || pattern.starts_with("../"))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
