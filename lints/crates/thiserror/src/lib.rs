#![feature(rustc_private)]

//! Rust review lints for thiserror API misuse.
//!
//! This group resolves source fields, display formatting, transparent errors,
//! backtrace attributes, and recursion patterns through one Dylint entry point.
//! Each constituent lint owns its semantic checks, UI fixture, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all thiserror-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent thiserror registration function.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = thiserror::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one thiserror-specific rule; this crate only groups them.
    thiserror_named_field_positional_format::register_lints(sess, lint_store);
    thiserror_no_std_path_display::register_lints(sess, lint_store);
    thiserror_raw_field_format::register_lints(sess, lint_store);
    thiserror_redundant_backtrace_attr::register_lints(sess, lint_store);
    thiserror_redundant_from_source::register_lints(sess, lint_store);
    // Register source-field and display-contract checks after format checks.
    thiserror_redundant_named_source::register_lints(sess, lint_store);
    thiserror_self_display_recursion::register_lints(sess, lint_store);
    thiserror_source_field_opt_out::register_lints(sess, lint_store);
    thiserror_transparent_single_source_display::register_lints(sess, lint_store);
    thiserror_tuple_format_positional_ambiguity::register_lints(sess, lint_store);
}
