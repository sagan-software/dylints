#![feature(rustc_private)]

//! Rust review lints for avoidable performance costs.
//!
//! This group registers diagnostics for allocation, boxing, ownership, and
//! call-cost patterns through one Dylint entry point. Each constituent lint
//! resolves its own semantic pattern, keeps its UI fixture, and emits a focused
//! recommendation without changing the target crate.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all performance lints in the group.
///
/// Dylint calls this entry point once per compilation. It forwards the compiler
/// session and lint store to each constituent registration function, then returns
/// after every performance lint is installed.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = perf::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    boxed_future_return::register_lints(sess, lint_store);
    expensive_as_method::register_lints(sess, lint_store);
    owned_input_field_clones::register_lints(sess, lint_store);
    ownership_at_boundaries::register_lints(sess, lint_store);
}
