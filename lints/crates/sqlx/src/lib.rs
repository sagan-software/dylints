#![feature(rustc_private)]

//! Rust review lints for `SQLx` API misuse.
//!
//! This group resolves query safety, row access, interpolation, pool limits,
//! query-builder, and empty-argument patterns through one Dylint entry point.
//! Each constituent lint owns its semantic checks, UI fixture, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all SQLx-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent `SQLx` registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = sqlx::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one SQLx-specific rule; this crate only groups them.
    sqlx_assert_sql_safe::register_lints(sess, lint_store);
    sqlx_empty_push_tuples::register_lints(sess, lint_store);
    sqlx_empty_push_values::register_lints(sess, lint_store);
    sqlx_panicking_row_get::register_lints(sess, lint_store);
    sqlx_panicking_statement_column::register_lints(sess, lint_store);
    // Finish with pool, query-builder, unchecked-macro, and connection-limit policies.
    sqlx_pool_connection_leak::register_lints(sess, lint_store);
    sqlx_query_builder_push_interpolation::register_lints(sess, lint_store);
    sqlx_query_builder_push_unseparated_interpolation::register_lints(sess, lint_store);
    sqlx_unchecked_query_macro::register_lints(sess, lint_store);
    sqlx_zero_max_connections::register_lints(sess, lint_store);
}
