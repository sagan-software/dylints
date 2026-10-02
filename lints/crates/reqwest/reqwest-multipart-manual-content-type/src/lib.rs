#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for manual content types on Reqwest multipart requests.
//!
//! This Dylint library resolves Reqwest multipart requests, reports manual
//! content-type headers, and recommends letting the multipart body set them.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use reqwest as _;

use reqwest_support::{emit_span_lint_with_help, is_content_type_header, reqwest_method_parts};
use rustc_hir::{Expr, ExprKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub REQWEST_MULTIPART_MANUAL_CONTENT_TYPE,
    Warn,
    "a Reqwest multipart request sets Content-Type manually",
    ReqwestMultipartManualContentType
}

impl<'tcx> LateLintPass<'tcx> for ReqwestMultipartManualContentType {
    /// Check either ordering of `header` and `multipart` in one fluent chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let manual_header_span =
            header_after_multipart(cx, expr).or_else(|| multipart_after_header(cx, expr));
        let Some(span) = manual_header_span else {
            return;
        };

        emit_span_lint_with_help(
            cx,
            REQWEST_MULTIPART_MANUAL_CONTENT_TYPE,
            span,
            "Reqwest must generate the multipart `Content-Type` with its boundary",
            "remove the manual `Content-Type` header from this builder chain",
        );
    }
}

/// Find a manual content type added after `.multipart(...)`.
fn header_after_multipart(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    let Some((receiver, [name, _value], span)) = reqwest_method_parts(cx, expr, "header") else {
        return None;
    };
    (is_content_type_header(cx, name) && chain_has_multipart(cx, receiver)).then_some(span)
}

/// Find a manual content type that precedes `.multipart(...)`.
fn multipart_after_header(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    let Some((receiver, [_form], _)) = reqwest_method_parts(cx, expr, "multipart") else {
        return None;
    };
    chain_content_type_header(cx, receiver)
}

/// Follow a fluent chain looking for a Reqwest multipart body setter.
fn chain_has_multipart(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    // Walk receiver links backward until the multipart call or chain root is reached.
    if reqwest_method_parts(cx, expr, "multipart").is_some() {
        return true;
    }
    let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
        return false;
    };
    chain_has_multipart(cx, receiver)
}

/// Follow a fluent chain looking for a manual content-type header.
fn chain_content_type_header(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    // Walk receiver links backward and return the nearest manual content-type header.
    if let Some((_receiver, [name, _value], span)) = reqwest_method_parts(cx, expr, "header")
        && is_content_type_header(cx, name)
    {
        return Some(span);
    }
    let ExprKind::MethodCall(_, receiver, _, _) = expr.kind else {
        return None;
    };
    chain_content_type_header(cx, receiver)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
