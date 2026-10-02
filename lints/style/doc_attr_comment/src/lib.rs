#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its suggestion"
)]

//! A lint to prefer line doc comments over simple `#[doc = "..."]` attributes.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::{AttrKind, AttrStyle, Attribute};
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, Lint, LintContext};
use rustc_span::{Span, sym};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub DOC_ATTR_COMMENT,
    Warn,
    "`doc` attribute can be a line doc comment",
    DocAttrComment
}

impl EarlyLintPass for DocAttrComment {
    /// Check attribute for this lint.
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        let Some(replacement) = simple_doc_comment_replacement(cx, attr) else {
            return;
        };

        emit_span_lint_with_suggestion(cx, DOC_ATTR_COMMENT, attr.span, attr.style, replacement);
    }
}

/// Helper for simple doc comment replacement analysis.
fn simple_doc_comment_replacement(cx: &EarlyContext<'_>, attr: &Attribute) -> Option<String> {
    // Rustc keeps `///`, `//!`, and block doc comments in `AttrKind::DocComment`;
    // only normal attributes can be replaceable explicit `doc` attributes.
    (!attr.span.from_expansion() && matches!(attr.kind, AttrKind::Normal(_)))
        .then_some(())
        .filter(|()| attr.has_name(sym::doc))
        // Require a direct, single-line string value before reading its source spelling.
        .and_then(|()| attr.value_str())
        .map(|doc| doc.as_str().to_owned())
        .filter(|doc| is_simple_line_doc_text(doc))
        .and_then(|doc| {
            cx.sess()
                .source_map()
                .span_to_snippet(attr.span)
                .ok()
                .filter(|source| is_direct_doc_attr_source(source, attr.style))
                .map(|_| doc)
        })
        .map(|doc| doc_comment_replacement(attr.style, &doc))
}

/// Return whether simple line doc text.
fn is_simple_line_doc_text(text: &str) -> bool {
    // Multiline and whitespace-sensitive strings may still be valid docs, but they need
    // human review before collapsing attribute syntax into a single comment line.
    !text.contains('\n') && !text.contains('\r') && text.trim() == text
}

/// Return whether direct doc attr source.
fn is_direct_doc_attr_source(source: &str, style: AttrStyle) -> bool {
    if source.contains('\n') || source.contains('\r') {
        return false;
    }

    // Rustc can normalize enabled `cfg_attr` and macro-valued doc attributes into
    // plain string-valued `doc` attributes. Source spelling is the remaining signal
    // that this attribute was directly written as `#[doc = "..."]`.
    let source = source.trim_start();
    if !match style {
        AttrStyle::Outer => source.starts_with("#[doc"),
        AttrStyle::Inner => source.starts_with("#![doc"),
    } {
        return false;
    }

    source
        .split_once('=')
        .is_some_and(|(_, value)| starts_with_string_literal(value.trim_start()))
}

/// Helper for starts with string literal analysis.
fn starts_with_string_literal(source: &str) -> bool {
    source.starts_with('"') || starts_with_raw_string_literal(source)
}

/// Helper for starts with raw string literal analysis.
fn starts_with_raw_string_literal(source: &str) -> bool {
    let Some(rest) = source.strip_prefix('r') else {
        return false;
    };

    rest.chars().find(|ch| *ch != '#') == Some('"')
}

/// Helper for doc comment replacement analysis.
fn doc_comment_replacement(style: AttrStyle, text: &str) -> String {
    // Select the marker that preserves outer versus inner documentation scope.
    let marker = match style {
        AttrStyle::Outer => "///",
        AttrStyle::Inner => "//!",
    };

    // Preserve empty documentation without adding trailing whitespace.
    if text.is_empty() {
        marker.to_owned()
    } else {
        format!("{marker} {text}")
    }
}

/// Emit the span lint with suggestion diagnostic.
fn emit_span_lint_with_suggestion(
    cx: &EarlyContext<'_>,
    lint: &'static Lint,
    span: Span,
    style: AttrStyle,
    replacement: String,
) {
    let (message, help) = match style {
        AttrStyle::Outer => (
            "outer `doc` attribute can be written as `///`",
            "write this as an outer line doc comment",
        ),
        AttrStyle::Inner => (
            "inner `doc` attribute can be written as `//!`",
            "write this as an inner line doc comment",
        ),
    };

    // The replacement covers exactly one source attribute, so rustc can apply it directly.
    cx.emit_span_lint(
        lint,
        span,
        DiagDecorator(move |diag| {
            let _ = diag.primary_message(message);
            let _ = diag.span_suggestion(span, help, replacement, Applicability::MachineApplicable);
        }),
    );
}

/// Helper for ui analysis.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
