#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for inert Clap `verbatim_doc_comment` attributes.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{ast_clap_attr, ast_has_doc, clap_ast_items, source_has_doc_before};
use rustc_ast::{Attribute, Crate};
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_VERBATIM_DOC_COMMENT_WITHOUT_DOC,
    Warn,
    "Clap verbatim_doc_comment has no effect without a doc comment",
    ClapDeriveVerbatimDocCommentWithoutDoc
}

impl EarlyLintPass for ClapDeriveVerbatimDocCommentWithoutDoc {
    /// Check items, variants, and fields in every cfg-active Clap derive.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Traverse every attribute-bearing target exposed by the shared AST model.
        for item in clap_ast_items(cx, krate) {
            // Check the item and its direct fields before descending into variants.
            check_target(cx, item.attrs);
            for field in item.fields {
                check_target(cx, field.attrs);
            }
            // Variants and their fields have independent helper and doc attributes.
            for variant in item.variants {
                check_target(cx, variant.attrs);
                for field in variant.fields {
                    check_target(cx, field.attrs);
                }
            }
        }
    }
}

/// Warn when one target requests doc preprocessing without providing docs.
fn check_target(cx: &EarlyContext<'_>, attrs: &[Attribute]) {
    // Accept either Clap helper namespace that exposes this setting.
    let verbatim = ast_clap_attr(cx, attrs, "command", "verbatim_doc_comment")
        .or_else(|| ast_clap_attr(cx, attrs, "arg", "verbatim_doc_comment"));
    let Some(verbatim) = verbatim else {
        return;
    };
    if ast_has_doc(attrs) || source_has_doc_before(cx, verbatim.span) {
        return;
    }

    // Report only after both structured and source-level doc checks fail.
    cx.emit_span_lint(
        CLAP_DERIVE_VERBATIM_DOC_COMMENT_WITHOUT_DOC,
        verbatim.span,
        DiagDecorator(|diagnostic| {
            let _configured_diagnostic = diagnostic
                .primary_message("`verbatim_doc_comment` has no doc comment to preserve")
                .help("add a doc comment, or remove `verbatim_doc_comment`");
        }),
    );
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
