#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check grouped Clap derive fields for an explicit value count.
//!
//! This Dylint library resolves Clap derive fields, reports vector fields
//! without an explicit count, and recommends declaring their accepted arity.
//!
//! The README defines the supported field types and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{
    ast_has_clap_attr, ast_is_special_clap_field, clap_arg_fields, clap_ast_items, clap_field_type,
};
use rustc_ast::Crate;
use rustc_errors::DiagDecorator;
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_VEC_VEC_WITHOUT_NUM_ARGS,
    Warn,
    "Clap derive grouped values need an explicit num_args boundary",
    ClapDeriveVecVecWithoutNumArgs
}

impl EarlyLintPass for ClapDeriveVecVecWithoutNumArgs {
    /// Check argument fields in every cfg-active Clap parser.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Restrict traversal to derives whose fields participate in argument parsing.
        for item in clap_ast_items(cx, krate) {
            if !item.derives.has_arg_fields() {
                continue;
            }

            for field in clap_arg_fields(&item) {
                // Exclude unsupported and explicitly configured fields before diagnosing.
                if !clap_field_type(field.ty).has_grouped_occurrences() {
                    continue;
                }
                if ast_is_special_clap_field(cx, field.attrs) {
                    continue;
                }
                if ast_has_clap_attr(cx, field.attrs, "arg", "num_args") {
                    continue;
                }

                // Anchor the diagnostic at the grouped field missing its value boundary.
                cx.emit_span_lint(
                    CLAP_DERIVE_VEC_VEC_WITHOUT_NUM_ARGS,
                    field.span,
                    DiagDecorator(|diagnostic| {
                        let _configured_diagnostic = diagnostic
                            .primary_message(
                                "this grouped Clap argument has no value-count boundary",
                            )
                            .help(
                                "add `#[arg(num_args = ...)]` to define the values in each occurrence",
                            );
                    }),
                );
            }
        }
    }
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
