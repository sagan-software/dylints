#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for unchecked `SQLx` query macros.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_errors;
extern crate rustc_hir;
extern crate rustc_span;

use std::collections::HashSet;

#[cfg(test)]
use sqlx as _;

use rustc_errors::{Applicability, DiagDecorator};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_span::Span;
use sqlx_support::{SqlxMacroCall, checked_query_macro_name, unchecked_query_macro};

/// Stateful pass used to emit one diagnostic per macro invocation.
///
/// The state deduplicates expanded HIR nodes while preserving the original
/// macro span needed for a machine-applicable checked-macro replacement.
#[derive(Debug, Default)]
pub struct SqlxUncheckedQueryMacro {
    /// Macro call sites already reported from expanded HIR nodes.
    reported_calls: HashSet<Span>,
}

dylint_support::documented_late_lint_with_pass! {
    #[doc = include_str!("../README.md")]
    pub SQLX_UNCHECKED_QUERY_MACRO,
    Warn,
    "SQLx query macro skips input or output type checking",
    SqlxUncheckedQueryMacro,
    SqlxUncheckedQueryMacro::default()
}

impl<'tcx> LateLintPass<'tcx> for SqlxUncheckedQueryMacro {
    /// Check the macro expansion origin and deduplicate its generated expressions.
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Resolve and deduplicate the unchecked macro before rendering its diagnostic.
        let Some((macro_call, checked_name)) = self.checked_macro_call(cx, expr.span) else {
            return;
        };
        let replacement =
            checked_macro_replacement(cx, macro_call.span, macro_call.name.as_str(), &checked_name);

        // Suggest the checked macro at the complete invocation span.
        cx.emit_span_lint(
            SQLX_UNCHECKED_QUERY_MACRO,
            macro_call.span,
            DiagDecorator(move |diagnostic| {
                let _configured_diagnostic = diagnostic.primary_message(format!(
                    "`{}!` skips SQLx input or output type checking",
                    macro_call.name
                ));
                // The checked macro verifies the query against the database at compile
                // time and can reject code that compiles today, so `cargo fix` must not
                // apply this rewrite automatically.
                if let Some(replacement) = replacement {
                    let _configured_suggestion = diagnostic.span_suggestion(
                        macro_call.span,
                        format!("use the checked `{checked_name}!` macro"),
                        replacement,
                        Applicability::MaybeIncorrect,
                    );
                } else {
                    let _configured_help =
                        diagnostic.help(format!("use the checked `{checked_name}!` macro"));
                }
            }),
        );
    }
}

impl SqlxUncheckedQueryMacro {
    /// Resolve one unchecked macro and retain only its checked counterpart.
    fn checked_macro_call(
        &mut self,
        cx: &LateContext<'_>,
        span: Span,
    ) -> Option<(SqlxMacroCall, String)> {
        // Resolve the macro expansion before deduplicating generated expressions.
        let macro_call = unchecked_query_macro(cx, span)?;
        if !self.reported_calls.insert(macro_call.span) {
            return None;
        }

        // Convert only the exact SQLx `_unchecked` naming convention.
        let checked_name = checked_query_macro_name(macro_call.name.as_str())?.to_owned();
        Some((macro_call, checked_name))
    }
}

/// Replace only the resolved macro name when its complete source span is available.
fn checked_macro_replacement(
    cx: &LateContext<'_>,
    span: Span,
    unchecked_name: &str,
    checked_name: &str,
) -> Option<String> {
    // Recover the exact source spelling before changing only the macro name.
    let source = cx.sess().source_map().span_to_snippet(span).ok()?;
    let (name_start, name_end) = checked_macro_name_bounds(&source, unchecked_name)?;
    // Preserve every token outside the resolved macro identifier.
    let mut replacement = source;
    replacement.replace_range(name_start..name_end, checked_name);
    Some(replacement)
}

/// Locate the exact unchecked macro identifier before its invocation bang.
fn checked_macro_name_bounds(source: &str, unchecked_name: &str) -> Option<(usize, usize)> {
    // Bound the replacement to the identifier immediately before the macro bang.
    let bang = source.find('!')?;
    let prefix = source.get(..bang)?.trim_end();
    let name_end = prefix.len();
    let name_start = name_end.checked_sub(unchecked_name.len())?;
    (prefix.get(name_start..) == Some(unchecked_name)).then_some((name_start, name_end))
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
