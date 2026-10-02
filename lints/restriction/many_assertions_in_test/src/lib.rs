#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores diagnostic builders"
)]

//! A lint to check for tests with many assertions.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

use std::collections::HashSet;

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::{
    Attribute, Body, Expr, ItemKind,
    attrs::AttributeKind,
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{ExpnKind, MacroKind, Span, SyntaxContext, def_id::LocalDefId};

/// `ASSERTION_THRESHOLD` configuration used by this lint.
const ASSERTION_THRESHOLD: usize = 4;

/// Diagnostic items of the standard assertion macros counted toward the threshold.
const ASSERTION_MACROS: &[&str] = &[
    "assert_macro",
    "assert_eq_macro",
    "assert_ne_macro",
    "debug_assert_macro",
    "debug_assert_eq_macro",
    "debug_assert_ne_macro",
];

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANY_ASSERTIONS_IN_TEST,
    Warn,
    "test has many assertions that could be a snapshot",
    ManyAssertionsInTest
}

impl<'tcx> LateLintPass<'tcx> for ManyAssertionsInTest {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        local_def_id: LocalDefId,
    ) {
        // Ignore closures and functions without built-in test semantics.
        if matches!(kind, FnKind::Closure) || !is_test_function(cx, local_def_id) {
            return;
        }

        // Count semantically resolved macro invocations after rustc expansion.
        let assertion_count = AssertionFinder::count(cx, body);

        if assertion_count >= ASSERTION_THRESHOLD {
            emit_span_lint_with_help(
                cx,
                MANY_ASSERTIONS_IN_TEST,
                span,
                format!("test has {assertion_count} assertions"),
                "use `insta` when many assertions describe one structured output contract",
            );
        }
    }
}

/// State used to collect distinct standard assertion macro invocations.
struct AssertionFinder<'cx, 'tcx> {
    /// Compiler context used to resolve each macro definition.
    cx: &'cx LateContext<'tcx>,
    /// Source call sites already counted for the current test.
    invocations: HashSet<Span>,
}

impl<'cx, 'tcx> AssertionFinder<'cx, 'tcx> {
    /// Count standard assertion macro invocations in one test body.
    fn count(cx: &'cx LateContext<'tcx>, body: &'tcx Body<'tcx>) -> usize {
        let mut finder = Self {
            cx,
            invocations: HashSet::new(),
        };

        // Expanded assertions produce several HIR expressions, so count their unique call sites.
        finder.visit_expr(body.value);
        finder.invocations.len()
    }
}

impl<'tcx> Visitor<'tcx> for AssertionFinder<'_, 'tcx> {
    /// Inspect one expression for a standard assertion expansion.
    fn visit_expr(&mut self, expr: &'tcx Expr<'tcx>) {
        // Record the call site once even when expansion nodes repeat.
        if let Some(invocation) = assertion_macro_invocation(self.cx, expr.span) {
            self.invocations.extend([invocation]);
        }

        intravisit::walk_expr(self, expr);
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: impl Into<String>,
    help: &'static str,
) {
    let message = message.into();

    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Return whether a function is a `#[test]` function in a `--test` build.
fn is_test_function(cx: &LateContext<'_>, local_def_id: LocalDefId) -> bool {
    // The test harness replaces `#[test]` with a same-named marker constant in the same module.
    let name = cx.tcx.item_name(local_def_id.to_def_id());
    let module = cx.tcx.parent_module_from_def_id(local_def_id);
    cx.tcx.hir_module_free_items(module).any(|item_id| {
        let item = cx.tcx.hir_item(item_id);
        matches!(item.kind, ItemKind::Const(ident, ..) if ident.name == name)
            && cx
                .tcx
                .hir_attrs(item.hir_id())
                .iter()
                .any(|attr| matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_))))
    })
}

/// Resolve one expression to its standard assertion macro call site.
fn assertion_macro_invocation(cx: &LateContext<'_>, span: Span) -> Option<Span> {
    let mut context = span.ctxt();

    // Walk outward because one assertion expansion creates several nested compiler expressions.
    while context != SyntaxContext::root() {
        let expansion = context.outer_expn_data();
        // Require a bang macro resolved to a standard-library assertion definition.
        if matches!(expansion.kind, ExpnKind::Macro(MacroKind::Bang, _))
            && let Some(def_id) = expansion.macro_def_id
            && cx
                .tcx
                .get_diagnostic_name(def_id)
                .is_some_and(|name| ASSERTION_MACROS.contains(&name.as_str()))
        {
            return Some(expansion.call_site.source_callsite());
        }

        let next = expansion.call_site.ctxt();
        // Stop when malformed expansion metadata cannot make outward progress.
        if next == context {
            break;
        }
        context = next;
    }

    None
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
