#![feature(rustc_private)]
#![expect(
    clippy::string_slice,
    clippy::let_underscore_must_use,
    reason = "rustc source spans identify builder expressions and diagnostics are configured in place"
)]

//! A lint to check for manual Debug implementations that could be derived.
//!
//! It inspects source structure and resolved rustc information to identify the
//! pattern described by the lint documentation. The implementation keeps
//! generated code and unsupported syntax conservative, then reports a focused
//! diagnostic so callers can choose the documented replacement with confidence.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use rustc_errors::DiagDecorator;
use rustc_hir::Item;
use rustc_lint::{LateContext, LateLintPass, Lint, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub MANUAL_DEBUG_IMPL,
    Warn,
    "manual `Debug` implementation could be derived",
    ManualDebugImpl
}

impl<'tcx> LateLintPass<'tcx> for ManualDebugImpl {
    /// Check item for this lint.
    fn check_item(&mut self, cx: &LateContext<'tcx>, item: &'tcx Item<'tcx>) {
        let Ok(source) = cx.sess().source_map().span_to_snippet(item.span) else {
            return;
        };

        if manual_debug_impl_can_derive(&source) {
            emit_span_lint_with_help(
                cx,
                MANUAL_DEBUG_IMPL,
                item.span,
                "manual `Debug` implementation looks derivable",
                "use `#[derive(Debug)]` unless the implementation intentionally redacts or customizes output",
            );
        }
    }
}

/// Helper for manual debug impl can derive analysis.
fn manual_debug_impl_can_derive(source: &str) -> bool {
    let Some(header) = source.split('{').next() else {
        return false;
    };
    if !debug_impl_header(header) || !source.contains("fn fmt(") || has_custom_marker(source) {
        return false;
    }

    // Keep the first version conservative: only builder calls that visibly finish are flagged.
    builder_finishes(source, ".debug_struct(") || builder_finishes(source, ".debug_tuple(")
}

/// Helper for debug impl header analysis.
fn debug_impl_header(header: &str) -> bool {
    header.contains(" Debug for ")
        || header.contains(" std::fmt::Debug for ")
        || header.contains("::Debug for ")
}

/// Helper for builder finishes analysis.
fn builder_finishes(source: &str, builder: &str) -> bool {
    let Some(builder_index) = source.find(builder) else {
        return false;
    };

    source[builder_index..].contains(".finish()")
}

/// Return whether custom marker is present.
fn has_custom_marker(source: &str) -> bool {
    let source = source.to_ascii_lowercase();

    ["redact", "secret", "<redacted>", "***", "if ", "match "]
        .iter()
        .any(|marker| source.contains(marker))
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
