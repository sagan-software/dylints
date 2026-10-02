#![feature(rustc_private)]

//! Rust review lints for suspicious code and error handling.
//!
//! This group registers diagnostics for broad error values, collection results,
//! conversion logging, and string-based error returns through one Dylint entry
//! point. Each constituent lint resolves its own semantic pattern and fixture.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all suspicious-code lints in deterministic order.
///
/// Dylint calls this entry point once per compilation. It forwards the compiler
/// session and lint store to every constituent registration function, then returns
/// after the group is installed.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = suspicious::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    broad_string_error_variant::register_lints(sess, lint_store);
    collection_bool_result::register_lints(sess, lint_store);
    logged_conversion_impl::register_lints(sess, lint_store);
    string_error_result::register_lints(sess, lint_store);
}
