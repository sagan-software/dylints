#![feature(rustc_private)]

//! Rust review lints for local policy restrictions.
//!
//! This group resolves semantic boundary, documentation, test, source-layout,
//! runtime, secret, and representation policies through one Dylint entry point.
//! Each constituent lint owns its thresholds, semantic checks, UI fixture,
//! configuration, and recommendation for the verified restriction.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all restriction lints in deterministic policy order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to focused helpers so each policy family remains
/// independently auditable.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = restriction::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register semantic type and metadata boundaries before source policies.
    register_restriction_domain_lints(sess, lint_store);
    register_restriction_source_lints(sess, lint_store);
}

/// Register type, path, logging, and documentation-boundary restrictions.
fn register_restriction_domain_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Start with numeric, country, and well-known-type boundaries.
    ambiguous_numeric_unit_field::register_lints(sess, lint_store);
    country_string_field::register_lints(sess, lint_store);
    custom_well_known_type::register_lints(sess, lint_store);
    // Register date and time boundary policies together.
    date_string_field::register_lints(sess, lint_store);
    datetime_integer_field::register_lints(sess, lint_store);
    // Continue with duration, path, and method boundaries.
    duration_integer_field::register_lints(sess, lint_store);
    hardcoded_path::register_lints(sess, lint_store);
    http_method_string::register_lints(sess, lint_store);
    insufficient_public_documentation::register_lints(sess, lint_store);
    interpolated_logging::register_lints(sess, lint_store);
    large_rust_crate::register_lints(sess, lint_store);
    large_rust_file::register_lints(sess, lint_store);
}

/// Register source, test, runtime, secret, and representation restrictions.
fn register_restriction_source_lints(
    sess: &rustc_session::Session,
    lint_store: &mut rustc_lint::LintStore,
) {
    // Start with test-case and source-intent policies.
    manual_test_cases::register_lints(sess, lint_store);
    many_assertions_in_test::register_lints(sess, lint_store);
    missing_intent_comments::register_lints(sess, lint_store);
    path_attribute_outside_root::register_lints(sess, lint_store);
    path_string_field::register_lints(sess, lint_store);
    public_serde_schema_derive::register_lints(sess, lint_store);
    repeated_cfg_gate::register_lints(sess, lint_store);
    // Finish with runtime input, secret, visibility, and URL boundaries.
    runtime_env_read::register_lints(sess, lint_store);
    secret_raw_type::register_lints(sess, lint_store);
    semantic_primitive_type::register_lints(sess, lint_store);
    struct_update_default::register_lints(sess, lint_store);
    unnecessary_module_directory::register_lints(sess, lint_store);
    unnecessary_public_type::register_lints(sess, lint_store);
    url_string_field::register_lints(sess, lint_store);
}
