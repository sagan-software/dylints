#![feature(rustc_private)]

//! Rust review lints for Strum enum representation contracts.
//!
//! This group resolves enum representation attributes and serialized-value
//! contracts through one Dylint entry point. The constituent lint owns its
//! semantic checks, UI fixture, configuration, and recommendation. The group
//! preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register the Strum-specific constituent lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to the constituent Strum registration function.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = strum::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    strum_enum_representation::register_lints(sess, lint_store);
}

#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
