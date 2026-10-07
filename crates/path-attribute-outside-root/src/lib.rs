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
use rustc_span::{FileName, Span, sym};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub PATH_ATTRIBUTE_OUTSIDE_ROOT,
    Warn,
    "`path` attribute outside the crate root file",
    PathAttributeOutsideRoot
}

impl EarlyLintPass for PathAttributeOutsideRoot {
    /// Check one source attribute against the crate root file.
    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        // Require the built-in string-valued form so unrelated attributes cannot match by name.
        if !attr.has_name(sym::path) || attr.value_str().is_none() {
            return;
        }

        if in_crate_root_file(cx, attr.span) {
            return;
        }

        emit_path_attribute_lint(cx, attr.span);
    }
}

/// Return whether an attribute is written in rustc's compiled crate-root file.
fn in_crate_root_file(cx: &EarlyContext<'_>, span: Span) -> bool {
    let FileName::Real(attr_file) = cx.sess().source_map().span_to_filename(span) else {
        // Virtual or generated sources have no file for this layout policy.
        return true;
    };

    // `src/main.rs`, `src/lib.rs`, `build.rs`, `src/bin/tool.rs`, and `tests/api.rs` are all
    // crate roots; a nested `lib.rs` module file is not.
    cx.sess()
        .local_crate_source_file()
        .is_some_and(|root| root == attr_file)
}

/// Emit the path-attribute policy diagnostic.
fn emit_path_attribute_lint(cx: &EarlyContext<'_>, span: Span) {
    // Explain the preferred layout without offering an unsafe automatic module move.
    cx.emit_span_lint(
        PATH_ATTRIBUTE_OUTSIDE_ROOT,
        span,
        DiagDecorator(|diag| {
            let _ = diag.primary_message("`path` attribute outside the crate root file");
            let _ = diag.help(
                "use Rust's standard module file layout, or declare this override in the crate root file",
            );
        }),
    );
}

/// Run the UI fixture suite.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
