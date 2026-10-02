#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]

//! A lint to check for non-trivial function bodies without intent comments.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_data_structures;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_lexer;
extern crate rustc_span;

use rustc_data_structures::fx::FxHashSet;
use rustc_errors::DiagDecorator;
use rustc_hir::{
    Block, Body, Stmt, StmtKind,
    intravisit::{FnKind, Visitor, walk_block, walk_stmt},
};
use rustc_lexer::{FrontmatterAllowed, TokenKind, tokenize};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MISSING_INTENT_COMMENTS,
    Warn,
    "function body does not have enough intent comments",
    MissingIntentComments
}

impl<'tcx> LateLintPass<'tcx> for MissingIntentComments {
    /// Check fn for this lint.
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _decl: &'tcx rustc_hir::FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _local_def_id: LocalDefId,
    ) {
        // Ignore support crates and closures because their statements are implementation details.
        if dylint_support::is_internal_support_crate(
            cx.tcx.crate_name(rustc_hir::def_id::LOCAL_CRATE),
        ) || matches!(kind, FnKind::Closure)
        {
            return;
        }

        // Count source-level statements before applying the minimum body threshold.
        let statement_count = body_statement_count(body);
        if statement_count < 5 {
            return;
        }

        // Compare line comments with one required comment per five statements.
        let snippet = cx.sess().source_map().span_to_snippet(body.value.span);
        let comment_count = snippet.as_deref().map_or(0, line_comment_count);
        let required_comments = statement_count.div_ceil(5);
        if comment_count >= required_comments {
            return;
        }

        // Report the measured and required counts at the complete function span.
        emit_span_lint_with_help(
            cx,
            MISSING_INTENT_COMMENTS,
            span,
            format!(
                "function body has {comment_count} intent comments but needs at least {required_comments} for {statement_count} statements"
            ),
            "add a short `//` comment explaining ordering, invariants, or side effects",
        );
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
    // Use rustc's native diagnostic decorator to keep diagnostics consistent.
    let message = message.into();
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Count the written statements and trailing expressions in a function body.
fn body_statement_count(body: &Body<'_>) -> usize {
    let mut counter = StatementCounter::default();
    counter.visit_expr(body.value);
    counter.statements + counter.macro_call_sites.len()
}

/// Counts source-level statements and trailing expressions in nested blocks.
///
/// A statement written in the function counts once. A statement produced by a
/// macro expansion counts once per macro call site, so `assert_eq!(a, b);`
/// counts as one statement rather than one per statement of its expansion.
#[derive(Default)]
struct StatementCounter {
    /// Written statements and trailing expressions observed in the function body.
    statements: usize,
    /// Distinct source call sites of macros whose expansion produced statements.
    macro_call_sites: FxHashSet<Span>,
}

impl StatementCounter {
    /// Record one statement-like node at `span`.
    fn record(&mut self, span: Span) {
        // Expanded code is attributed to its outermost call site in the written source.
        if span.from_expansion() {
            let _is_new = self.macro_call_sites.insert(span.source_callsite());
        } else {
            self.statements += 1;
        }
    }
}

impl<'hir> Visitor<'hir> for StatementCounter {
    /// Count each non-item statement before inspecting nested expressions.
    fn visit_stmt(&mut self, statement: &'hir Stmt<'hir>) {
        if !matches!(statement.kind, StmtKind::Item(_)) {
            self.record(statement.span);
        }
        walk_stmt(self, statement);
    }

    /// Count a block's trailing expression and continue into every nested block.
    fn visit_block(&mut self, block: &'hir Block<'hir>) {
        if let Some(expr) = block.expr {
            self.record(expr.span);
        }
        walk_block(self, block);
    }
}

/// Count the source lines that hold a `//` line comment.
///
/// Comments are not part of HIR, so the body text is lexed.
///
/// `//` inside a string literal, such as a URL, does not count, and each line
/// counts once.
fn line_comment_count(source: &str) -> usize {
    // Track the logical source line and the last line that produced a comment.
    let mut line = 0_usize;
    let mut last_comment_line = None;
    let mut count = 0_usize;

    // Walk tokens in order, advancing the line number by the newlines inside each token.
    let mut offset = 0_usize;
    for token in tokenize(source, FrontmatterAllowed::No) {
        // Stop when token metadata cannot map back to a source slice.
        let Some(text) = usize::try_from(token.len)
            .ok()
            .and_then(|len| source.get(offset..offset.saturating_add(len)))
        else {
            break;
        };
        // Count the first line-comment token on each logical line.
        if matches!(token.kind, TokenKind::LineComment { .. }) && last_comment_line != Some(line) {
            last_comment_line = Some(line);
            count += 1;
        }
        line += text.matches('\n').count();
        offset += text.len();
    }
    count
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}

/// Count each commented line once and ignore `//` inside string literals.
#[test]
fn counts_lexed_line_comments() {
    let source = "{\n    // one\n    let url = \"https://example.com\"; // two\n    /* block */\n}";
    assert_eq!(line_comment_count(source), 2);
    assert_eq!(line_comment_count("let text = \"a // b\";"), 0);
}
