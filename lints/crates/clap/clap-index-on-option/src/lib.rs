#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks positional indices on clap options.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;

#[cfg(test)]
use clap as _;
use rustc_errors::DiagDecorator;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_INDEX_ON_OPTION,
    Warn,
    "a clap option has a positional index",
    ClapIndexOnOption
}

impl<'tcx> LateLintPass<'tcx> for ClapIndexOnOption {
    /// Check one outermost resolved clap argument chain.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(span) = clap_support::index_on_option(cx, expr) else {
            return;
        };
        cx.emit_span_lint(
            CLAP_INDEX_ON_OPTION,
            span,
            DiagDecorator(|diagnostic| {
                let _configured_diagnostic = diagnostic
                    .primary_message("`index` only configures positional arguments")
                    .help("remove `index`, or make this argument positional");
            }),
        );
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
