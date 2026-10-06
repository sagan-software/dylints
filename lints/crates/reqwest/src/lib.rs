#![feature(rustc_private)]

//! Rust review lints for Reqwest API misuse.
//!
//! This group resolves request loops, blocking calls, client ownership, cookie
//! providers, retries, multipart headers, and TLS settings through one Dylint
//! entry point. Each constituent lint owns its semantic checks and fixture.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Reqwest-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Reqwest registration function.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = reqwest_lints::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Reqwest-specific rule; this crate only groups them.
    reqwest_blocking_in_async::register_lints(sess, lint_store);
    reqwest_client_in_loop::register_lints(sess, lint_store);
    reqwest_client_wrapped_in_shared_pointer::register_lints(sess, lint_store);
    reqwest_cookie_provider_overridden::register_lints(sess, lint_store);
    reqwest_get_in_loop::register_lints(sess, lint_store);
    // Finish with multipart, retry-budget, and TLS policies.
    reqwest_multipart_manual_content_type::register_lints(sess, lint_store);
    reqwest_retry_invalid_max_extra_load::register_lints(sess, lint_store);
    reqwest_retry_no_budget::register_lints(sess, lint_store);
    reqwest_tls_danger_invalid_certs::register_lints(sess, lint_store);
    reqwest_tls_danger_invalid_hostnames::register_lints(sess, lint_store);
}
