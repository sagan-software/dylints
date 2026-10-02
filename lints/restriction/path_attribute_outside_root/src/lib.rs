#![feature(rustc_private)]
#![expect(
    clippy::let_underscore_must_use,
    reason = "the lint intentionally ignores the diagnostic builder after emitting its message"
)]
#![warn(unused_extern_crates)]

//! A lint to check for path attributes outside conventional crate root files.
//!
//! It examines source attributes that redirect a module or include to another
//! path, resolves the local file boundary, and reports paths that escape the
//! conventional crate-root layout. The rule keeps explicit generated or
//! external-source exceptions separate from ordinary source organization.

extern crate rustc_ast;
extern crate rustc_errors;
extern crate rustc_span;

use rustc_ast::Attribute;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};
use rustc_span::{Span, sym};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub PATH_ATTRIBUTE_OUTSIDE_ROOT,
    Warn,
    "`path` attribute outside a conventional crate root file",
    PathAttributeOutsideRoot
}

impl EarlyLintPass for PathAttributeOutsideRoot {
    /// Check one source attribute against the allowed root filenames.
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        // Require the built-in string-valued form so unrelated attributes cannot match by name.
        if !attr.has_name(sym::path) || attr.value_str().is_none() {
            return;
        }

        if is_allowed_source_file(cx, attr.span) {
            return;
        }

        emit_path_attribute_lint(cx, attr.span);
    }
}

/// Return whether an attribute belongs to an allowed source filename.
fn is_allowed_source_file(cx: &EarlyContext<'_>, span: Span) -> bool {
    let Some(path) = cx
        .sess()
        .source_map()
        .span_to_filename(span)
        .into_local_path()
    else {
        // Virtual or remapped sources have no stable filename for this local policy.
        return true;
    };

    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| matches!(name, "main.rs" | "lib.rs" | "build.rs"))
}

/// Emit the path-attribute policy diagnostic.
fn emit_path_attribute_lint(cx: &EarlyContext<'_>, span: Span) {
    // Explain the preferred layout without offering an unsafe automatic module move.
    cx.emit_span_lint(
        PATH_ATTRIBUTE_OUTSIDE_ROOT,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message("`path` attribute outside a conventional crate root file");
            let _ = diag.help(
                "use Rust's standard module file layout, or declare this override from an allowed root file",
            );
        }),
    );
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
