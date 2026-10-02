#![feature(rustc_private)]

//! Rust review lints for clap API misuse.
//!
//! This group resolves command, derive, parser, option, and external-subcommand
//! patterns through one Dylint entry point. Each constituent lint owns its
//! semantic checks, UI fixture, configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Clap-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Clap registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = clap::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one clap-specific rule; this crate only groups them.
    clap_allow_hyphen_values_without_num_args::register_lints(sess, lint_store);
    clap_allow_negative_numbers_without_num_args::register_lints(sess, lint_store);
    clap_command_with_arguments_option::register_lints(sess, lint_store);
    clap_derive_author_without_help_template::register_lints(sess, lint_store);
    clap_derive_bool_default_true::register_lints(sess, lint_store);
    // Continue with derive-action, parser, and vector-shape policies.
    clap_derive_redundant_action::register_lints(sess, lint_store);
    clap_derive_redundant_value_parser::register_lints(sess, lint_store);
    clap_derive_vec_vec_without_num_args::register_lints(sess, lint_store);
    clap_derive_verbatim_doc_comment_without_doc::register_lints(sess, lint_store);
    // Register external-command and positional-option policies next.
    clap_external_subcommand_parser_without_external_subcommands::register_lints(sess, lint_store);
    clap_index_on_option::register_lints(sess, lint_store);
    clap_last_option::register_lints(sess, lint_store);
    clap_multicall_no_binary_name::register_lints(sess, lint_store);
    clap_require_equals_without_num_args::register_lints(sess, lint_store);
    clap_required_conditional_conflict::register_lints(sess, lint_store);
    // Finish with trailing variadic-argument validation.
    clap_trailing_var_arg_without_num_args::register_lints(sess, lint_store);
}
