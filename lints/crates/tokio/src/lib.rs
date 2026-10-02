#![feature(rustc_private)]

//! Rust review lints for Tokio API misuse.
//!
//! This group resolves blocking calls, runtime construction, sleep loops,
//! spawning, channels, intervals, and thread-count patterns through one Dylint
//! entry point. Each constituent lint owns its semantic checks and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Tokio-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Tokio registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = tokio::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Tokio-specific rule; this crate only groups them.
    tokio_blocking_call_in_async::register_lints(sess, lint_store);
    tokio_handle_block_on_in_async::register_lints(sess, lint_store);
    tokio_runtime_block_on_in_async::register_lints(sess, lint_store);
    tokio_runtime_new_in_async::register_lints(sess, lint_store);
    tokio_sleep_in_loop::register_lints(sess, lint_store);
    // Finish with task, channel, interval, and thread-count policies.
    tokio_spawn_blocking_async_closure::register_lints(sess, lint_store);
    tokio_unbounded_channel::register_lints(sess, lint_store);
    tokio_zero_capacity_channel::register_lints(sess, lint_store);
    tokio_zero_duration_interval::register_lints(sess, lint_store);
    tokio_zero_runtime_thread_count::register_lints(sess, lint_store);
}
