#![feature(rustc_private)]

//! Rust review lints for tracing API misuse.
//!
//! This group resolves spans, async scope, field formatting, interpolation, and
//! current-span patterns through one Dylint entry point. Each constituent lint
//! owns its semantic checks, UI fixture, configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all tracing-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent tracing registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = tracing::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one tracing-specific rule; this crate only groups them.
    tracing_async_block_in_sync_scope::register_lints(sess, lint_store);
    tracing_await_holding_span_guard::register_lints(sess, lint_store);
    tracing_current_span_enter::register_lints(sess, lint_store);
    tracing_current_span_or_current::register_lints(sess, lint_store);
    tracing_direct_record_all::register_lints(sess, lint_store);
    // Continue with field formatting, instrumentation, and interpolation checks.
    tracing_format_field::register_lints(sess, lint_store);
    tracing_instrument_current_span::register_lints(sess, lint_store);
    tracing_message_interpolation::register_lints(sess, lint_store);
    tracing_none_enter::register_lints(sess, lint_store);
    tracing_none_or_current::register_lints(sess, lint_store);
    // Finish with optional-span and redundant-field checks.
    tracing_none_record::register_lints(sess, lint_store);
    tracing_redundant_field_assignment::register_lints(sess, lint_store);
    tracing_redundant_in_current_span::register_lints(sess, lint_store);
    tracing_to_string_field::register_lints(sess, lint_store);
}
