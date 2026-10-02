#![feature(rustc_private)]

//! Rust review lints for Serde API misuse.
//!
//! This group resolves borrowing, defaults, portability, attributes, naming,
//! serialization, and untagged-enum patterns through one Dylint entry point.
//! Each constituent lint owns its semantic checks, UI fixture, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Serde-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Serde registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = serde::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Serde-specific rule; this crate only groups them.
    serde_all_fields_default::register_lints(sess, lint_store);
    serde_borrow_redundant_str_bytes::register_lints(sess, lint_store);
    serde_cow_missing_borrow::register_lints(sess, lint_store);
    serde_deserialize_any_portability::register_lints(sess, lint_store);
    serde_expecting_style::register_lints(sess, lint_store);
    // Continue with fallback and attribute-combination policies.
    serde_fallback_missing_other::register_lints(sess, lint_store);
    serde_flatten_deny_unknown_fields::register_lints(sess, lint_store);
    serde_inert_directional_attr::register_lints(sess, lint_store);
    serde_manual_rename_all::register_lints(sess, lint_store);
    serde_serialize_str_to_string::register_lints(sess, lint_store);
    // Finish with serialization and untagged-enum policies.
    serde_skip_serializing_roundtrip::register_lints(sess, lint_store);
    serde_skip_serializing_variant_error::register_lints(sess, lint_store);
    serde_untagged_missing_expecting::register_lints(sess, lint_store);
}
