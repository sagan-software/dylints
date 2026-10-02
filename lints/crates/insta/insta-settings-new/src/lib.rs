#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks `Settings::new`.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Insta settings constructors, reports creation
//! that discards inherited settings, and recommends cloning current settings.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

#[cfg(test)]
use insta as _;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;

dylint_support::documented_late_lint! {
    #[doc = include_str!("../README.md")]
    pub INSTA_SETTINGS_NEW,
    Warn,
    "Insta Settings::new discards current settings",
    InstaSettingsNew
}

impl<'tcx> LateLintPass<'tcx> for InstaSettingsNew {
    /// Check the resolved Settings constructor.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !insta_support::is_settings_new_call(cx, expr) {
            return;
        }
        let replacement = settings_clone_current_replacement(cx, expr.span);
        cx.emit_span_lint(
            INSTA_SETTINGS_NEW,
            expr.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic =
                    diagnostic.primary_message("`Settings::new` discards inherited configuration");
                if let Some(replacement) = replacement {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        expr.span,
                        "use `Settings::clone_current`",
                        replacement,
                        Applicability::MachineApplicable,
                    );
                } else {
                    let _configured_help = diagnostic.help("use `Settings::clone_current`");
                }
            }),
        );
    }
}

/// Replace the resolved zero-argument constructor while preserving its source path.
fn settings_clone_current_replacement(cx: &LateContext<'_>, span: Span) -> Option<String> {
    // Recover the exact source spelling before changing only the constructor name.
    let source = cx.sess().source_map().span_to_snippet(span).ok()?;
    let (method_start, method_end) = settings_method_bounds(&source)?;
    // Preserve every token outside the resolved method identifier.
    let mut replacement = source;
    replacement.replace_range(method_start..method_end, "clone_current");
    Some(replacement)
}

/// Locate a standalone `new` method name in a zero-argument constructor call.
fn settings_method_bounds(source: &str) -> Option<(usize, usize)> {
    let opening = source.find('(')?;
    let prefix = source.get(..opening)?.trim_end();
    let method_start = prefix.rfind("new")?;
    (prefix.get(method_start..) == Some("new")).then_some((method_start, prefix.len()))
}

#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
