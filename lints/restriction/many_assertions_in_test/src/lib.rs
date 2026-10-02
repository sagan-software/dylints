#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and uses rustc-provided UTF-8 byte boundaries"
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
    Attribute, Body, Expr,
    attrs::AttributeKind,
    intravisit::{self, FnKind, Visitor},
};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{ExpnKind, MacroKind, Span, SyntaxContext, def_id::LocalDefId, sym};

/// `ASSERTION_THRESHOLD` configuration used by this lint.
const ASSERTION_THRESHOLD: usize = 4;

/// Standard assertion macros that contribute to the snapshot-test threshold.
const ASSERTION_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
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
        if matches!(kind, FnKind::Closure) || !test_function(cx, local_def_id, span) {
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

/// Helper for test function analysis.
fn test_function(cx: &LateContext<'_>, local_def_id: LocalDefId, span: Span) -> bool {
    let hir_id = cx.tcx.local_def_id_to_hir_id(local_def_id);
    let attrs: &[Attribute] = cx.tcx.hir_attrs(hir_id);

    // Rustc strips built-in test attributes before late linting, so retain a narrow source fallback.
    attrs.iter().any(|attr| {
        attr.has_name(sym::test)
            || matches!(attr, Attribute::Parsed(AttributeKind::RustcTestMarker(_)))
    }) || source_has_builtin_test_attr(cx, span)
}

/// Return whether adjacent source attributes include the built-in `#[test]` marker.
fn source_has_builtin_test_attr(cx: &LateContext<'_>, span: Span) -> bool {
    // Read the source file because late HIR no longer retains the built-in marker.
    let source_file = cx.sess().source_map().lookup_source_file(span.lo());
    let Some(source) = source_file.src.as_deref() else {
        return false;
    };
    let Ok(item_start) = usize::try_from((span.lo() - source_file.start_pos).0) else {
        return false;
    };
    if item_start > source.len() {
        return false;
    }

    // Start immediately before the item and ignore intervening whitespace.
    let mut cursor = skip_whitespace_backward(source, item_start);

    // Walk only adjacent outer attributes so comments and earlier items cannot create a match.
    while cursor > 0 && source.as_bytes().get(cursor - 1) == Some(&b']') {
        let end = cursor - 1;
        let Some(start) = source[..end].rfind("#[") else {
            break;
        };
        // Accept only the exact built-in test attribute body.
        if source
            .get(start + 2..end)
            .is_some_and(|body| body.trim() == "test")
        {
            return true;
        }
        cursor = skip_whitespace_backward(source, start);
    }

    false
}

/// Move a source cursor backward over ASCII whitespace.
fn skip_whitespace_backward(source: &str, mut cursor: usize) -> usize {
    // Attribute discovery starts at the previous non-whitespace byte.
    while cursor > 0
        && source
            .as_bytes()
            .get(cursor - 1)
            .is_some_and(u8::is_ascii_whitespace)
    {
        cursor -= 1;
    }

    cursor
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
            && matches!(cx.tcx.crate_name(def_id.krate).as_str(), "core" | "std")
            && ASSERTION_MACROS.contains(&cx.tcx.item_name(def_id).as_str())
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
