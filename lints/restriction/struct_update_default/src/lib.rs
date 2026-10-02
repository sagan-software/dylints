#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "rustc diagnostic builder results are configured through side effects"
)]
#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "unsupported rustc expression and resolution variants stay outside this narrow shape check"
)]
#![warn(unused_extern_crates)]

//! A lint to check for struct update syntax with `Default`.
//!
//! It identifies a resolved `Default::default()` base hidden behind struct
//! update syntax and points at the source expression that conceals fields.
//! The diagnostic keeps construction explicit so future readers can see every
//! initialized field and review changes to the type's defaults directly.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;
use rustc_errors::DiagDecorator;
use rustc_hir::{Expr, ExprKind, StructTailExpr, def::Res};
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub STRUCT_UPDATE_DEFAULT,
    Warn,
    "struct update syntax hides defaulted fields",
    StructUpdateDefault
}

impl<'tcx> LateLintPass<'tcx> for StructUpdateDefault {
    /// Check struct update bases for the resolved `Default::default` method.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let Some(base_span) = default_base_span(cx, expr) {
            // Point at the hidden default base that should become explicit fields.
            cx.emit_span_lint(
                STRUCT_UPDATE_DEFAULT,
                base_span,
                DiagDecorator(|diag| {
                    let _ = diag.primary_message(
                        "struct update syntax hides fields behind `Default::default()`",
                    );
                    let _ = diag.help("set every field explicitly at the construction boundary");
                }),
            );
        }
    }
}

/// Returns the span of a resolved `Default::default()` struct-update base.
fn default_base_span(cx: &LateContext<'_>, expr: &Expr<'_>) -> Option<Span> {
    match expr.kind {
        ExprKind::Struct(_, _, StructTailExpr::Base(base)) => Some(base),
        _ => None,
    }
    .and_then(|base| match base.kind {
        ExprKind::Call(callee, []) => Some((base, callee)),
        _ => None,
    })
    .and_then(|(base, callee)| match callee.kind {
        ExprKind::Path(qpath) => Some((base, callee, qpath)),
        _ => None,
    })
    .and_then(
        |(base, callee, qpath)| match cx.typeck_results().qpath_res(&qpath, callee.hir_id) {
            Res::Def(_, def_id) => Some((base, def_id)),
            _ => None,
        },
    )
    .filter(|(_, def_id)| {
        cx.tcx
            .def_path_str(*def_id)
            .ends_with("default::Default::default")
    })
    .map(|(base, _)| base.span)
}

/// Run the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
