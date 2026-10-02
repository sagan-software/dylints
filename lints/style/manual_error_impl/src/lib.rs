#![feature(rustc_private)]
#![expect(
    clippy::string_slice,
    clippy::let_underscore_must_use,
    reason = "rustc source offsets identify function bodies and diagnostics are configured in place"
)]

//! A lint to check for manual `Error` and `Display` implementations that could
//! use thiserror.
//!
//! It inspects source-authored error types, separates formatting and source
//! forwarding behavior, and reports implementations whose generated behavior
//! matches a supported thiserror derive. The parser ignores comments while
//! retaining source spans so the recommendation stays attached to real code.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::BTreeMap;

use rustc_errors::DiagDecorator;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_ERROR_IMPL,
    Warn,
    "manual `Error` and `Display` implementations could use `thiserror`",
    ManualErrorImpl,
    ManualErrorImpl::default()
}

/// Stateful pass that pairs nearby `Display` and `Error` impls by local type.
#[derive(Default)]
struct ManualErrorImpl {
    /// Candidate impl state keyed by the local type name being implemented.
    impls_by_type: BTreeMap<String, ErrorImplCandidate>,
}

/// Partial evidence gathered for one candidate local error type.
#[derive(Default)]
struct ErrorImplCandidate {
    /// Span of a simple manual `Display` impl, if one has been seen.
    display_span: Option<Span>,
    /// Whether a plain empty `Error` impl has been seen.
    saw_plain_error_impl: bool,
    /// Whether a custom `Error` impl has been seen and should suppress the lint.
    saw_custom_error_impl: bool,
    /// Whether this candidate has already emitted a lint.
    emitted: bool,
}

/// Classification for an implementation block that participates in this lint.
enum ImplKind {
    /// Simple `Display` impl on a named local type.
    SimpleDisplay {
        /// Local type name implemented by this impl block.
        type_name: String,
    },
    /// Empty `Error` impl on a named local type.
    PlainError {
        /// Local type name implemented by this impl block.
        type_name: String,
    },
    /// Non-empty `Error` impl on a named local type.
    CustomError {
        /// Local type name implemented by this impl block.
        type_name: String,
    },
}

/// Trait implemented by a candidate impl block.
enum TraitKind {
    /// `std::fmt::Display`.
    Display,
    /// `std::error::Error`.
    Error,
}

impl<'tcx> LateLintPass<'tcx> for ManualErrorImpl {
    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Ok(source) = cx.sess().source_map().span_to_snippet(item.span) else {
            return;
        };
        let Some(kind) = impl_kind(&source) else {
            return;
        };

        let type_name = match &kind {
            ImplKind::SimpleDisplay { type_name }
            | ImplKind::PlainError { type_name }
            | ImplKind::CustomError { type_name } => type_name,
        };
        let candidate = self.impls_by_type.entry(type_name.clone()).or_default();

        // Track only the impl shapes that can safely be replaced by a derive.
        // Accumulate Display and Error evidence across separate impl items.
        match kind {
            ImplKind::SimpleDisplay { .. } => candidate.display_span = Some(item.span),
            ImplKind::PlainError { .. } => candidate.saw_plain_error_impl = true,
            ImplKind::CustomError { .. } => candidate.saw_custom_error_impl = true,
        }

        if candidate.ready_to_emit()
            && let Some(display_span) = candidate.display_span
        {
            candidate.emitted = true;
            emit_span_lint_with_help(
                cx,
                MANUAL_ERROR_IMPL,
                display_span,
                "manual `Error` and `Display` implementations look derivable",
                "use `#[derive(thiserror::Error, Debug)]` with a `#[error(\"...\")]` message",
            );
        }
    }
}

impl ErrorImplCandidate {
    /// Helper for ready to emit analysis.
    const fn ready_to_emit(&self) -> bool {
        self.display_span.is_some()
            && self.saw_plain_error_impl
            && !self.saw_custom_error_impl
            && !self.emitted
    }
}

/// Helper for impl kind analysis.
fn impl_kind(source: &str) -> Option<ImplKind> {
    // Parse the impl header before classifying its body behavior.
    let header = source.split('{').next()?;
    let trait_kind = trait_kind(header)?;
    let type_name = local_type_name(header)?;

    match trait_kind {
        TraitKind::Display => {
            simple_display_impl(source).then_some(ImplKind::SimpleDisplay { type_name })
        }
        TraitKind::Error if plain_error_impl(source) => Some(ImplKind::PlainError { type_name }),
        TraitKind::Error => Some(ImplKind::CustomError { type_name }),
    }
}

/// Helper for trait kind analysis.
fn trait_kind(header: &str) -> Option<TraitKind> {
    let (before_for, _) = header.rsplit_once(" for ")?;
    let trait_path = before_for.split_whitespace().last()?;
    let trait_name = trait_path.rsplit("::").next()?;

    match trait_name {
        "Display" => Some(TraitKind::Display),
        "Error" => Some(TraitKind::Error),
        _ => None,
    }
}

/// Return the local type name.
fn local_type_name(header: &str) -> Option<String> {
    let (_, after_for) = header.rsplit_once(" for ")?;
    let raw_type = after_for.split_whitespace().next()?;
    let type_name = raw_type.rsplit("::").next()?;

    // Keep this lint to simple named local types where derive placement is obvious.
    // Reject generic, reference, tuple, and otherwise structured self types.
    let has_only_ident_chars = type_name.chars().all(ident_char);
    let starts_with_alpha = type_name.chars().next()?.is_ascii_alphabetic();
    if has_only_ident_chars && starts_with_alpha {
        Some(type_name.to_owned())
    } else {
        None
    }
}

/// Helper for ident char analysis.
const fn ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// Helper for simple display impl analysis.
fn simple_display_impl(source: &str) -> bool {
    let Some(body) = body_after_fn(source, "fn fmt") else {
        return false;
    };
    if display_body_has_custom_marker(body) {
        return false;
    }

    // Diagnose only a single expression-style write macro that maps to `#[error]`.
    // Reject semicolon-terminated or otherwise multi-statement formatting bodies.
    let body = body.trim().trim_end_matches('}').trim();
    write_macro_count(body) == 1
        && starts_with_write_macro(body)
        && !body.contains(';')
        && body.contains("write")
}

/// Helper for display body has custom marker analysis.
fn display_body_has_custom_marker(body: &str) -> bool {
    let lower = body.to_ascii_lowercase();

    ["if ", "match ", "redact", "secret", "<redacted>", "***"]
        .iter()
        .any(|marker| lower.contains(marker))
}

/// Helper for starts with write macro analysis.
fn starts_with_write_macro(body: &str) -> bool {
    body.starts_with("write!(") || body.starts_with("write !")
}

/// Helper for write macro count analysis.
fn write_macro_count(body: &str) -> usize {
    body.matches("write!(").count() + body.matches("write !").count()
}

/// Helper for plain error impl analysis.
fn plain_error_impl(source: &str) -> bool {
    let Some(body) = impl_body(source) else {
        return false;
    };
    if body.contains("fn ") || body.contains("fn\n") {
        return false;
    }

    // An empty impl with comments still has no custom `Error` behavior.
    strip_comments(body).trim().is_empty()
}

/// Helper for body after fn analysis.
fn body_after_fn<'a>(source: &'a str, fn_name: &str) -> Option<&'a str> {
    let fn_start = source.find(fn_name)?;
    let body_start = source[fn_start..].find('{')? + fn_start;

    source.get(body_start + 1..)
}

/// Helper for impl body analysis.
fn impl_body(source: &str) -> Option<&str> {
    let body_start = source.find('{')?;
    let body_end = source.rfind('}')?;

    source.get(body_start + 1..body_end)
}

/// Comment delimiter recognized by the source scanner.
#[derive(Clone, Copy)]
enum CommentKind {
    /// A comment that ends at the next newline.
    Line,
    /// A comment that ends at the next closing delimiter.
    Block,
}

/// Helper for strip comments analysis.
fn strip_comments(source: &str) -> String {
    // Scan source once while preserving every non-comment character.
    let mut stripped = String::new();
    let mut chars = source.chars().peekable();

    // Preserve non-comment source so the emptiness check ignores explanatory comments.
    while let Some(ch) = chars.next() {
        if let Some(kind) = comment_kind(ch, chars.peek().copied()) {
            // Consume the second delimiter character before scanning comment contents.
            let _ = chars.next();
            consume_comment(kind, &mut chars, &mut stripped);
        } else {
            // Copy ordinary source characters without normalization.
            stripped.push(ch);
        }
    }

    stripped
}

/// Identifies a comment delimiter without consuming either character.
fn comment_kind(ch: char, next: Option<char>) -> Option<CommentKind> {
    (ch == '/').then_some(next).and_then(|next| match next {
        Some('/') => Some(CommentKind::Line),
        Some('*') => Some(CommentKind::Block),
        _ => None,
    })
}

/// Removes one comment body and preserves a line-comment newline.
fn consume_comment(
    kind: CommentKind,
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    stripped: &mut String,
) {
    match kind {
        CommentKind::Line => consume_line_comment(chars, stripped),
        CommentKind::Block => consume_block_comment(chars),
    }
}

/// Consumes a line comment through its newline separator.
fn consume_line_comment(
    chars: &mut std::iter::Peekable<std::str::Chars<'_>>,
    stripped: &mut String,
) {
    // Preserve the newline so later emptiness checks retain source separation.
    for next in chars.by_ref() {
        if next == '\n' {
            stripped.push('\n');
            break;
        }
    }
}

/// Consumes a block comment through its first closing delimiter.
fn consume_block_comment(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    // Search for the first closing delimiter without interpreting nested text.
    while let Some(next) = chars.next() {
        // Consume both closing characters when the block ends.
        if next == '*' && chars.peek() == Some(&'/') {
            let _ = chars.next();
            break;
        }
    }
}

/// Emit the span lint with help diagnostic.
fn emit_span_lint_with_help(
    cx: &LateContext<'_>,
    lint: &'static Lint,
    span: Span,
    message: &'static str,
    help: &'static str,
) {
    // Use rustc's native diagnostic decorator to match the rest of this lint suite.
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
