#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! A lint to check for value parsers already inferred by Clap derive.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_ast;
extern crate rustc_errors;

#[cfg(test)]
use clap as _;

use clap_support::{
    ast_clap_attr, ast_clap_attr_entry_source, ast_clap_attr_single_entry, ast_has_clap_attr,
    ast_is_special_clap_field, ast_type_source, clap_arg_fields, clap_ast_items,
    clap_value_parser_type, compact_source,
};
use rustc_ast::Crate;
use rustc_errors::{Applicability, DiagDecorator};
use rustc_lint::{EarlyContext, EarlyLintPass, LintContext};

dylint_support::documented_early_lint! {
    #[doc = include_str!("../README.md")]
    pub CLAP_DERIVE_REDUNDANT_VALUE_PARSER,
    Warn,
    "Clap derive already infers this value_parser call from the field type",
    ClapDeriveRedundantValueParser
}

impl EarlyLintPass for ClapDeriveRedundantValueParser {
    /// Check argument fields in every cfg-active Clap derive.
    fn check_crate(&mut self, cx: &EarlyContext<'_>, krate: &Crate) {
        // Visit only derives that can contain Clap argument fields.
        for item in clap_ast_items(cx, krate) {
            if !item.derives.has_arg_fields() {
                continue;
            }

            // Ignore special fields and value enums with distinct parser rules.
            for field in clap_arg_fields(&item) {
                if ast_is_special_clap_field(cx, field.attrs)
                    || ast_has_clap_attr(cx, field.attrs, "arg", "value_enum")
                {
                    continue;
                }

                // Compare the configured parser with the parser inferred from the type.
                let Some(parser_source) =
                    ast_clap_attr_entry_source(cx, field.attrs, "arg", "value_parser")
                else {
                    continue;
                };
                let parser_type = clap_value_parser_type(field.ty);
                let Some(type_source) = ast_type_source(cx, parser_type) else {
                    continue;
                };
                if !is_inferred_parser(&parser_source, &type_source) {
                    continue;
                }
                let Some(parser) = ast_clap_attr(cx, field.attrs, "arg", "value_parser") else {
                    continue;
                };
                let fix_span = ast_clap_attr_single_entry(cx, field.attrs, "arg", "value_parser")
                    .map(|attr| attr.span);

                // Point at the resolved attribute entry that the user can remove.
                cx.emit_span_lint(
                    CLAP_DERIVE_REDUNDANT_VALUE_PARSER,
                    parser.span,
                    DiagDecorator(move |diagnostic| {
                        let _configured_diagnostic = diagnostic.primary_message(
                            "Clap derive already selects this parser from the field type",
                        );
                        if let Some(fix_span) = fix_span {
                            let _configured_suggestion = diagnostic.span_suggestion(
                                fix_span,
                                "remove the redundant `value_parser` setting",
                                String::new(),
                                Applicability::MachineApplicable,
                            );
                        } else {
                            let _configured_help =
                                diagnostic.help("remove the redundant `value_parser` setting");
                        }
                    }),
                );
            }
        }
    }
}

/// Match an unmodified `value_parser!` call for the inferred element type.
fn is_inferred_parser(parser_source: &str, type_source: &str) -> bool {
    // Remove insignificant whitespace from both source fragments.
    let parser_source = compact_source(parser_source);
    let type_source = compact_source(type_source);

    [
        // Accept the imported macro path.
        format!("value_parser=value_parser!({type_source})"),
        // Accept the crate-qualified macro path.
        format!("value_parser=clap::value_parser!({type_source})"),
        // Accept the absolute crate-qualified macro path.
        format!("value_parser=::clap::value_parser!({type_source})"),
    ]
    .contains(&parser_source)
}

/// Run the UI fixture.
#[test]
fn ui() {
    dylint_testing::ui_test_examples(env!("CARGO_PKG_NAME"));
}
