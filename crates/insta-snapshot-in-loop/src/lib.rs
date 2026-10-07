#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks snapshot assertions repeated by source-level loops.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use insta as _;

use insta_support::{
    insta_macro_invocation, is_in_allow_duplicates, is_in_loop, is_insta_function, is_insta_type,
    snapshot_macro_names, string_literal,
};
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SNAPSHOT_IN_LOOP,
    Warn,
    "an Insta snapshot assertion is repeated in a loop without allow_duplicates",
    InstaSnapshotInLoop
}

impl<'tcx> LateLintPass<'tcx> for InstaSnapshotInLoop {
    /// Check the runtime assertion call that every snapshot macro expands to.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve Insta's runtime assertion and the public macro that produced it.
        let Some((snapshot, invocation)) = snapshot_argument(cx, expr).and_then(|snapshot| {
            Some((
                snapshot,
                insta_macro_invocation(cx, expr, snapshot_macro_names())?,
            ))
        }) else {
            return;
        };
        // A name computed on each pass can name a different snapshot every time.
        if !is_in_loop(cx, expr, invocation.span)
            || is_in_allow_duplicates(cx, expr)
            || !has_repeated_name(cx, snapshot)
        {
            return;
        }
        // Each macro invocation expands to exactly one runtime assertion call.
        cx.emit_span_lint(
            INSTA_SNAPSHOT_IN_LOOP,
            invocation.span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("this assertion can visit the same snapshot more than once")
                    .help(
                        "use distinct snapshot names or wrap equal repeats in `allow_duplicates!`",
                    );
            }),
        );
    }
}

/// Return the snapshot value argument of a call to Insta's runtime assertion.
fn snapshot_argument<'tcx>(
    cx: &LateContext<'tcx>,
    expr: &'tcx Expr<'tcx>,
) -> Option<&'tcx Expr<'tcx>> {
    if let ExprKind::Call(callee, [snapshot, ..]) = expr.kind
        && is_insta_function(cx, callee, "assert_snapshot")
    {
        Some(snapshot)
    } else {
        None
    }
}

/// Return whether every pass gives the snapshot the same name.
///
/// Insta's macros pass `(name, content).into()` or a `BinarySnapshotValue`. An
/// automatic name, an inline snapshot, or a literal name repeats on each pass.
/// A computed name can differ between passes, so it does not count.
fn has_repeated_name(cx: &LateContext<'_>, snapshot: &Expr<'_>) -> bool {
    // Peel the `.into()` conversion around the generated value.
    let ExprKind::MethodCall(_, value, [], _) = snapshot.kind else {
        return true;
    };
    // Inspect the generated value before classifying the snapshot name.
    let name = if let ExprKind::Tup([name, _]) = value.kind {
        Some(name)
    } else if let ExprKind::Struct(_, fields, _) = value.kind {
        fields
            .iter()
            .find(|field| field.ident.name.as_str() == "name_and_extension")
            .map(|field| field.expr)
    } else {
        None
    };
    name.is_none_or(|name| {
        let name_ty = cx.typeck_results().expr_ty(name).peel_refs();
        is_insta_type(cx, name_ty, "AutoName")
            || is_insta_type(cx, name_ty, "InlineValue")
            || string_literal(cx, name).is_some()
    })
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
