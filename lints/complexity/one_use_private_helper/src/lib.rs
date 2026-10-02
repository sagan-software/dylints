#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    clippy::string_slice,
    reason = "the lint intentionally ignores diagnostic builders and uses rustc-provided UTF-8 byte boundaries"
)]

//! A lint to check for one-use private expression helpers.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::{fs::File, io::Read, path::PathBuf};

use rustc_errors::DiagDecorator;
use rustc_hir::{Body, Expr, ExprKind, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::{Span, def_id::LocalDefId};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub ONE_USE_PRIVATE_HELPER,
    Warn,
    "private one-expression helper is only called once",
    OneUsePrivateHelper
}

impl<'tcx> LateLintPass<'tcx> for OneUsePrivateHelper {
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
        if one_use_candidate(cx, kind, body, span, local_def_id).is_some() {
            emit_span_lint_with_help(
                cx,
                ONE_USE_PRIVATE_HELPER,
                span,
                "private one-expression helper is only called once",
                "inline the expression at the call site unless the helper names validation, a hook, fixture setup, or a domain rule",
            );
        }
    }
}

/// Return the source evidence for one diagnostic candidate.
fn one_use_candidate<'tcx>(
    cx: &LateContext<'tcx>,
    kind: FnKind<'tcx>,
    body: &Body<'tcx>,
    span: Span,
    local_def_id: LocalDefId,
) -> Option<(String, String)> {
    // Keep each source and semantic prerequisite in one composable filter.
    private_item_function_name(cx, kind, span, local_def_id)
        .filter(|name| single_expression_body(body) && !excluded_helper_name(name))
        .and_then(|name| {
            cx.sess()
                .source_map()
                .span_to_snippet(span)
                .ok()
                .filter(|source| !self_referential_body(source, &name))
                .map(|_| name)
        })
        .and_then(|name| {
            same_file_source(cx, span)
                .filter(|source| direct_call_count(source, &name) == 1)
                .map(|source| (name, source))
        })
}

/// Return the private item function name.
fn private_item_function_name(
    cx: &LateContext<'_>,
    kind: FnKind<'_>,
    span: Span,
    local_def_id: LocalDefId,
) -> Option<String> {
    // Restrict the analysis to named free functions.
    let FnKind::ItemFn(ident, ..) = kind else {
        return None;
    };

    // Exclude generated and attributed functions whose lifecycle may be implicit.
    let from_expansion = span.from_expansion();
    let has_attrs = !cx
        .tcx
        .hir_attrs(cx.tcx.local_def_id_to_hir_id(local_def_id))
        .is_empty();
    if from_expansion || has_attrs {
        return None;
    }

    let source = cx.sess().source_map().span_to_snippet(span).ok()?;
    simple_private_function_source(&source, ident.name.as_str())
        .then(|| ident.name.to_ident_string())
}

/// Return source text for simple private function.
fn simple_private_function_source(source: &str, name: &str) -> bool {
    let Some(after_fn) = source.trim_start().strip_prefix("fn ") else {
        return false;
    };
    let Some(after_name) = after_fn.strip_prefix(name) else {
        return false;
    };

    // Requiring `(` immediately after the name skips generic helpers and other unusual item forms.
    after_name.trim_start().starts_with('(')
}

/// Helper for single expression body analysis.
const fn single_expression_body(body: &Body<'_>) -> bool {
    // Require a statement-free block with one inlineable tail expression.
    let ExprKind::Block(block, _) = body.value.kind else {
        return false;
    };
    let Some(expr) = block.expr else {
        return false;
    };

    block.stmts.is_empty() && simple_inlineable_expression(expr)
}

/// Helper for simple inlineable expression analysis.
const fn simple_inlineable_expression(expr: &Expr<'_>) -> bool {
    !matches!(
        expr.kind,
        ExprKind::If(..)
            | ExprKind::Match(..)
            | ExprKind::Loop(..)
            | ExprKind::Closure(..)
            | ExprKind::Block(..)
            | ExprKind::Ret(..)
            | ExprKind::Break(..)
    )
}

/// Return the excluded helper name.
fn excluded_helper_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let words = lower
        .split('_')
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();

    // Names that encode validation, tests, framework hooks, construction roles, or domain
    // predicates carry review value even when a first pass sees only one direct call.
    words.iter().any(|word| {
        matches!(
            *word,
            "validate"
                | "validation"
                | "valid"
                | "check"
                | "ensure"
                | "assert"
                | "test"
                | "tests"
                | "fixture"
                | "fixtures"
                | "build"
                | "builder"
                | "make"
                | "hook"
                | "rule"
                | "policy"
                | "invariant"
        )
    }) || matches!(
        words.first().copied(),
        Some("can" | "should" | "is" | "has" | "must")
    )
}

/// Helper for self referential body analysis.
fn self_referential_body(function_source: &str, name: &str) -> bool {
    direct_call_count(function_source, name) > 0
}

/// Return whether file source match.
fn same_file_source(cx: &LateContext<'_>, span: Span) -> Option<String> {
    let path = local_source_path(cx, span)?;
    let mut source = String::new();

    // Reading the file keeps same-file counting independent of partial function snippets.
    let mut file = File::open(path).ok()?;
    let _ = file.read_to_string(&mut source).ok()?;

    Some(source)
}

/// Helper for local source path analysis.
fn local_source_path(cx: &LateContext<'_>, span: Span) -> Option<PathBuf> {
    cx.sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
}

/// Helper for direct call count analysis.
fn direct_call_count(source: &str, name: &str) -> usize {
    // Scan exact name occurrences while advancing beyond each candidate.
    let mut count = 0;
    let mut index = 0;

    while let Some(relative_start) = source[index..].find(name) {
        let start = index + relative_start;
        let end = start + name.len();

        // Count only unqualified call syntax at identifier boundaries.
        if looks_like_direct_call(source, start, end) {
            count += 1;
        }

        // Resume after the matched name to guarantee forward progress.
        index = end;
    }

    count
}

/// Return whether the source looks like like direct call.
fn looks_like_direct_call(source: &str, start: usize, end: usize) -> bool {
    identifier_boundary_before(source, start)
        && identifier_boundary_after(source, end)
        && next_non_ws_char(source, end) == Some('(')
        && !definition_site(source, start)
        && !qualified_call(source, start)
}

/// Helper for identifier boundary before analysis.
fn identifier_boundary_before(source: &str, start: usize) -> bool {
    previous_char(source, start).is_none_or(|ch| !is_ident_continue(ch))
}

/// Helper for identifier boundary after analysis.
fn identifier_boundary_after(source: &str, end: usize) -> bool {
    source[end..]
        .chars()
        .next()
        .is_none_or(|ch| !is_ident_continue(ch))
}

/// Return the previous char around a source position.
fn previous_char(source: &str, byte_index: usize) -> Option<char> {
    source[..byte_index].chars().next_back()
}

/// Return the next non ws char around a source position.
fn next_non_ws_char(source: &str, byte_index: usize) -> Option<char> {
    source[byte_index..].chars().find(|ch| !ch.is_whitespace())
}

/// Helper for definition site analysis.
fn definition_site(source: &str, start: usize) -> bool {
    previous_token(source, start).is_some_and(|token| token == "fn")
}

/// Helper for qualified call analysis.
fn qualified_call(source: &str, start: usize) -> bool {
    previous_non_ws_char(source, start).is_some_and(|ch| matches!(ch, '.' | ':'))
}

/// Return the previous non ws char around a source position.
fn previous_non_ws_char(source: &str, byte_index: usize) -> Option<char> {
    source[..byte_index]
        .chars()
        .rev()
        .find(|ch| !ch.is_whitespace())
}

/// Return the previous token around a source position.
fn previous_token(source: &str, byte_index: usize) -> Option<&str> {
    let before = source[..byte_index].trim_end();
    let start = before
        .rfind(|ch: char| !is_ident_continue(ch))
        .map_or(0, |index| index + ch_len_at(before, index));

    before.get(start..)
}

/// Helper for ch len at analysis.
fn ch_len_at(source: &str, byte_index: usize) -> usize {
    source[byte_index..]
        .chars()
        .next()
        .map_or(0, char::len_utf8)
}

/// Return whether ident continue.
const fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep diagnostics consistent with the suite.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message(message);
            let _ = diag.help(help);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
