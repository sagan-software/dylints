#![feature(rustc_private)]

//! Rust review lints for Axum routing API misuse.
//!
//! This group resolves route, nesting, path, and service-layer patterns through
//! one Dylint entry point. Each constituent lint owns its semantic checks, UI
//! fixture, configuration, and recommendation for the verified Axum misuse.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Axum-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Axum registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = axum::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Axum-specific rule; this crate only groups them.
    axum_nest_at_root::register_lints(sess, lint_store);
    axum_nest_service_at_root::register_lints(sess, lint_store);
    axum_nest_wildcard_path::register_lints(sess, lint_store);
    axum_route_empty_path::register_lints(sess, lint_store);
    axum_route_layer_on_empty_router::register_lints(sess, lint_store);
    // Finish with legacy capture, path, and route-service policies.
    axum_route_legacy_colon_capture::register_lints(sess, lint_store);
    axum_route_legacy_wildcard_capture::register_lints(sess, lint_store);
    axum_route_path_missing_slash::register_lints(sess, lint_store);
    axum_route_service_empty_path::register_lints(sess, lint_store);
    axum_route_service_router::register_lints(sess, lint_store);
}
