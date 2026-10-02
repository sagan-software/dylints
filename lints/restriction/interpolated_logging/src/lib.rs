#![feature(rustc_private)]

//! A lint to check for interpolated logging messages.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{
    MacCall,
    token::{LitKind, Token, TokenKind},
    tokenstream::TokenTree,
};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_pre_expansion_lint! {
    #[doc = include_str!("../README.md")]
    pub INTERPOLATED_LOGGING,
    Warn,
    "logging message uses string interpolation instead of structured fields",
    InterpolatedLogging
}

impl EarlyLintPass for InterpolatedLogging {
    /// Check mac for this lint.
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        if !logging_or_output_macro(mac) {
            return;
        }

        // Inspect the raw macro tokens before expansion so both `log` and `tracing` macros work.
        if let Some(span) = interpolated_string_span(mac) {
            emit_span_lint_with_help(
                cx,
                INTERPOLATED_LOGGING,
                span,
                "logging message uses string interpolation",
                "put runtime values in structured key/value fields and keep the message static",
            );
        }
    }
}

/// Emit the interpolated-logging diagnostic with a concrete migration hint.
fn emit_span_lint_with_help(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to keep the lint dependency-free.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(|diag| {
            let _configured_message = diag.primary_message(message);
            let _configured_help = diag.help(help);
        }),
    );
}

/// Return whether a macro call has a conventional logging or formatted-output name.
fn logging_or_output_macro(mac: &MacCall) -> bool {
    mac.path.segments.last().is_some_and(|segment| {
        matches!(
            segment.ident.name.as_str(),
            "trace"
                | "debug"
                | "info"
                | "warn"
                | "error"
                | "event"
                | "print"
                | "println"
                | "eprint"
                | "eprintln"
                | "write"
                | "writeln"
        )
    })
}

/// Return the first string literal span that contains an unescaped format placeholder.
fn interpolated_string_span(mac: &MacCall) -> Option<Span> {
    // The first interpolated string is enough to explain the logging call pattern.
    mac.args
        .tokens
        .iter()
        .find_map(interpolated_string_token_span)
}

/// Search a token tree recursively for an interpolated string literal.
fn interpolated_string_token_span(tree: &TokenTree) -> Option<Span> {
    match tree {
        TokenTree::Token(
            Token {
                kind: TokenKind::Literal(lit),
                span,
            },
            _,
        ) if matches!(lit.kind, LitKind::Str | LitKind::StrRaw(_))
            && format_string_has_placeholder(lit.symbol.as_str()) =>
        {
            Some(*span)
        }
        TokenTree::Delimited(_, _, _, tokens) => {
            // Recurse into nested token trees so helper macro arguments stay visible.
            tokens.iter().find_map(interpolated_string_token_span)
        }
        TokenTree::Token(..) => None,
    }
}

/// Return whether a format string contains an unescaped opening brace.
fn format_string_has_placeholder(message: &str) -> bool {
    // Scan characters so escaped braces remain distinguishable from placeholders.
    let mut chars = message.chars().peekable();

    while let Some(ch) = chars.next() {
        // Ignore ordinary characters until an opening brace appears.
        if ch != '{' {
            continue;
        }

        // Consume doubled opening braces because they render as literal text.
        if chars.peek().is_some_and(|next| *next == '{') {
            let _ = chars.next();
            continue;
        }

        return true;
    }

    false
}

/// Verify that the documented structured tracing calls compile.
#[test]
fn tracing_examples_compile() {
    use tracing::{info, warn};

    let user_id = 42;
    let request_id = "req-7";
    info!(user_id, "user logged in");
    warn!(request_id, "request failed");
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
