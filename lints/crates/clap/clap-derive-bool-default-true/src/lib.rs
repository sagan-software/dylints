#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for Clap `SetTrue` flags whose default is already true.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{
    ClapFieldType, ast_clap_attr, ast_clap_attr_entry_source, ast_is_special_clap_field,
    clap_arg_fields, clap_ast_items, clap_field_type, compact_source,
};
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_BOOL_DEFAULT_TRUE,
    Warn,
    "Clap SetTrue flags cannot change a value that defaults to true",
    ClapDeriveBoolDefaultTrue
}

impl EarlyLintPass for ClapDeriveBoolDefaultTrue {
    /// Check boolean argument fields in every cfg-active Clap parser.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Restrict traversal to derives whose fields participate in argument parsing.
        for item in clap_ast_items(cx, krate) {
            if !item.derives.has_arg_fields() {
                continue;
            }

            // Keep ordinary boolean flags that use the inferred `SetTrue` action.
            for field in clap_arg_fields(&item) {
                if clap_field_type(field.ty) != ClapFieldType::Bool
                    || ast_is_special_clap_field(cx, field.attrs)
                    || !uses_set_true(cx, field.attrs)
                {
                    continue;
                }
                let Some(default) = true_default_attr(cx, field.attrs) else {
                    continue;
                };

                // Anchor the diagnostic at the explicit true default attribute.
                cx.emit_span_lint(
                    CLAP_DERIVE_BOOL_DEFAULT_TRUE,
                    default.span,
                    DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message(
                                "this `SetTrue` flag is already true when the option is absent",
                            )
                            .help(
                                "remove the true default, or use a negative flag with `ArgAction::SetFalse`",
                            );
                    }),
                );
            }
        }
    }
}

/// Return whether Clap will use its inferred `SetTrue` action.
fn uses_set_true(cx: &EarlyContext<'_>, attrs: &[rustc_ast::Attribute]) -> bool {
    let Some(action) = ast_clap_attr_entry_source(cx, attrs, "arg", "action") else {
        return true;
    };
    matches!(
        compact_source(&action).as_str(),
        "action=ArgAction::SetTrue"
            | "action=clap::ArgAction::SetTrue"
            | "action=::clap::ArgAction::SetTrue"
    )
}

/// Find a literal true default on one argument field.
fn true_default_attr<'attr>(
    cx: &EarlyContext<'_>,
    attrs: &'attr [rustc_ast::Attribute],
) -> Option<&'attr rustc_ast::Attribute> {
    // Check typed and string default forms against their exact literal source.
    for (key, expected) in [
        ("default_value_t", "default_value_t=true"),
        ("default_value", "default_value=\"true\""),
    ] {
        let Some(source) = ast_clap_attr_entry_source(cx, attrs, "arg", key) else {
            continue;
        };
        // Return the structured attribute only after its bounded source matches.
        if compact_source(&source) == expected {
            return ast_clap_attr(cx, attrs, "arg", key);
        }
    }

    None
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
