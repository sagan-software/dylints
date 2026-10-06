#![feature(rustc_private)]

//! Rust review lints for likely incorrect behavior.
//!
//! This group registers diagnostics for error propagation and process-boundary
//! failures through one Dylint entry point. Each constituent lint resolves its
//! own semantic pattern, keeps its UI fixture, and emits the recommendation
//! that matches the verified behavior.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all correctness lints in the group.
///
/// Dylint calls this entry point once per compilation. It forwards the compiler
/// session and lint store to each constituent registration function, then returns
/// after both lints are installed.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = correctness::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    logged_error_continue::register_lints(sess, lint_store);
    panic_in_main::register_lints(sess, lint_store);
}
