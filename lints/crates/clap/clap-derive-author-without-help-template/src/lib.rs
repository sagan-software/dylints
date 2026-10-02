#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for hidden author metadata on Clap-derived commands.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.
//!
//! This Dylint library resolves Clap derive metadata, reports author settings
//! without the matching help template, and recommends documenting the author.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{ast_clap_attr, ast_has_clap_attr, clap_ast_items};
use rustc_ast::{Attribute, Crate};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_AUTHOR_WITHOUT_HELP_TEMPLATE,
    Warn,
    "Clap derive author metadata is hidden by the default help template",
    ClapDeriveAuthorWithoutHelpTemplate
}

impl EarlyLintPass for ClapDeriveAuthorWithoutHelpTemplate {
    /// Check command targets in every cfg-active Clap-derived item.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Check each item before its independently configurable enum variants.
        for item in clap_ast_items(cx, krate) {
            check_command_target(cx, item.attrs);
            // Variant command attributes do not inherit the item's help template.
            for variant in item.variants {
                check_command_target(cx, variant.attrs);
            }
        }
    }
}

/// Warn when an author is configured without the template needed to display it.
fn check_command_target(cx: &EarlyContext<'_>, attrs: &[Attribute]) {
    // Report author metadata only when no custom template can display it.
    let Some(author) = ast_clap_attr(cx, attrs, "command", "author") else {
        return;
    };
    if ast_has_clap_attr(cx, attrs, "command", "help_template") {
        return;
    }

    cx.emit_span_lint(
        CLAP_DERIVE_AUTHOR_WITHOUT_HELP_TEMPLATE,
        author.span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic
                .primary_message(
                    "Clap's default help template does not display this `author` value",
                )
                .help("add a custom `help_template` containing `{author}`, or remove `author`");
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
