#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for binary Insta snapshot names without file extensions.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use insta_support::{is_insta_type, string_literal};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_BINARY_SNAPSHOT_MISSING_EXTENSION,
    Warn,
    "a binary Insta snapshot name has no file extension",
    InstaBinarySnapshotMissingExtension
}

impl<'tcx> LateLintPass<'tcx> for InstaBinarySnapshotMissingExtension {
    /// Check the binary snapshot value constructed by Insta's resolved assertion macro.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve the generated value and recover its source snapshot name.
        let Some(name_span) = binary_snapshot_name_span(cx, expr) else {
            return;
        };

        // Report at the macro call site instead of inside expanded code.
        cx.emit_span_lint(
            INSTA_BINARY_SNAPSHOT_MISSING_EXTENSION,
            name_span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this binary snapshot name has no file extension")
                    .help("include an extension, such as `.bin`, in the snapshot name");
            }),
        );
    }
}

/// Resolve a missing-extension binary snapshot name to its source call site.
fn binary_snapshot_name_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<rustc_span::Span> {
    // Require the expanded Insta value constructor and its resolved definition.
    let ExprKind::Struct(_, fields, _) = expr.kind else {
        return None;
    };
    if !is_insta_type(cx, cx.typeck_results().expr_ty(expr), "BinarySnapshotValue") {
        return None;
    }

    // Recover the user's original snapshot name from the generated field.
    let name = fields
        .iter()
        .find(|field| field.ident.name.as_str() == "name_and_extension")
        .map(|field| field.expr)?;
    let value = string_literal(cx, name)?;

    // Reject names that already carry an extension.
    (!value.contains('.')).then_some(name.span.source_callsite())
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
