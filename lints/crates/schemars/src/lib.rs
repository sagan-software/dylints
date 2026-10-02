#![feature(rustc_private)]

//! Rust review lints for Schemars API misuse.
//!
//! This group resolves schema references, recursion, serde attributes, and
//! schema-generation choices through one Dylint entry point. Each constituent
//! lint owns its semantic checks, UI fixture, configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Schemars-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Schemars registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = schemars::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Schemars-specific rule; this crate only groups them.
    schemars_json_schema_ref_return::register_lints(sess, lint_store);
    schemars_recursive_inline_schema::register_lints(sess, lint_store);
    schemars_redundant_serde_default::register_lints(sess, lint_store);
    schemars_redundant_serde_deny_unknown_fields::register_lints(sess, lint_store);
    schemars_redundant_serde_rename::register_lints(sess, lint_store);
    // Finish with rename, skip, tag, transparent, and schema-generation policies.
    schemars_redundant_serde_rename_all::register_lints(sess, lint_store);
    schemars_redundant_serde_skip::register_lints(sess, lint_store);
    schemars_redundant_serde_tag::register_lints(sess, lint_store);
    schemars_redundant_serde_transparent::register_lints(sess, lint_store);
    schemars_schema_for_value_json_schema::register_lints(sess, lint_store);
}
