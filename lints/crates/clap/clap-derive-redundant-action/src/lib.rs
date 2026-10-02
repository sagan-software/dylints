#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for argument actions already inferred by Clap derive.
//! This Dylint library resolves the named API or syntax at compile time,
//! reports a source diagnostic for the undesired or redundant pattern, and
//! leaves unrelated code unchanged. Its README defines the checked boundary,
//! the recommended replacement, and the UI fixture that protects behavior.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{
    ast_clap_attr, ast_clap_attr_entry_source, ast_clap_attr_single_entry,
    ast_is_special_clap_field, clap_arg_fields, clap_ast_items, clap_field_type, compact_source,
};
use rustc_ast::Crate;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_REDUNDANT_ACTION,
    Warn,
    "Clap derive already infers this ArgAction from the field type",
    ClapDeriveRedundantAction
}

impl EarlyLintPass for ClapDeriveRedundantAction {
    /// Check argument fields in every cfg-active Clap derive.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Visit only derives that can contain Clap argument fields.
        for item in clap_ast_items(cx, krate) {
            if !item.derives.has_arg_fields() {
                continue;
            }

            // Ignore command and subcommand fields because their actions differ.
            for field in clap_arg_fields(&item) {
                if ast_is_special_clap_field(cx, field.attrs) {
                    continue;
                }

                // Compare the configured action with the action inferred from the type.
                let Some(action_source) =
                    ast_clap_attr_entry_source(cx, field.attrs, "arg", "action")
                else {
                    continue;
                };
                let expected = clap_field_type(field.ty).inferred_action();
                if !is_exact_action(&action_source, expected) {
                    continue;
                }
                let Some(action) = ast_clap_attr(cx, field.attrs, "arg", "action") else {
                    continue;
                };
                let fix_span = ast_clap_attr_single_entry(cx, field.attrs, "arg", "action")
                    .map(|attr| attr.span);

                // Point at the resolved attribute entry that the user can remove.
                cx.emit_span_lint(
                    CLAP_DERIVE_REDUNDANT_ACTION,
                    action.span,
                    DiagDecorator(move |diagnostic| {
                        let _configured_diagnostic = diagnostic.primary_message(format!(
                            "Clap already infers `ArgAction::{expected}` from this field type"
                        ));
                        if let Some(fix_span) = fix_span {
                            let _configured_suggestion = diagnostic.span_suggestion(
                                fix_span,
                                "remove the redundant `action` setting",
                                String::new(),
                                Applicability::MachineApplicable,
                            );
                        } else {
                            let _configured_help =
                                diagnostic.help("remove the redundant `action` setting");
                        }
                    }),
                );
            }
        }
    }
}

/// Match the common direct paths to one `ArgAction` variant.
fn is_exact_action(source: &str, expected: &str) -> bool {
    // Remove insignificant whitespace before comparing attribute spellings.
    let source = compact_source(source);
    [
        // Accept the imported enum path.
        format!("action=ArgAction::{expected}"),
        // Accept the crate-qualified enum path.
        format!("action=clap::ArgAction::{expected}"),
        // Accept the absolute crate-qualified enum path.
        format!("action=::clap::ArgAction::{expected}"),
    ]
    .contains(&source)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
