#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for binary Insta snapshot names without file extensions.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;

use rustc_ast::LitKind;
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, def::Res};
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
    let ExprKind::Struct(path, fields, _) = expr.kind else {
        return None;
    };
    if !is_binary_snapshot_value(cx, path, expr.hir_id) {
        return None;
    }

    // Recover the user's original snapshot name from the generated field.
    let name = fields
        .iter()
        .find(|field| field.ident.name.as_str() == "name_and_extension")
        .map(|field| field.expr)?;
    let value = literal_string(name)?;

    // Reject names that already carry an extension.
    (!value.as_str().contains('.')).then_some(name.span.source_callsite())
}

/// Return whether a resolved constructor is Insta's binary snapshot value.
fn is_binary_snapshot_value(
    cx: &LateContext<'_>,
    path: &rustc_hir::QPath<'_>,
    hir_id: rustc_hir::HirId,
) -> bool {
    // Resolve the constructor definition before inspecting its generated fields.
    let Res::Def(_, def_id) = cx.typeck_results().qpath_res(path, hir_id) else {
        return false;
    };
    cx.tcx.crate_name(def_id.krate).as_str() == "insta"
        && cx
            .tcx
            .def_path_str(def_id)
            .ends_with("::BinarySnapshotValue")
}

/// Extract a string literal from one generated HIR expression.
const fn literal_string(expr: &Expr<'_>) -> Option<rustc_span::Symbol> {
    // Restrict source recovery to a literal node with a string value.
    let ExprKind::Lit(literal) = expr.kind else {
        return None;
    };
    let LitKind::Str(value, _) = literal.node else {
        return None;
    };
    Some(value)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
