#![feature(rustc_private)]

//! Rust review lints for Insta snapshot API misuse.
//!
//! This group resolves snapshot paths, filters, settings, formats, and async
//! scope patterns through one Dylint entry point. Each constituent lint owns its
//! semantic checks, UI fixture, configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Insta-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent Insta registration function.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = insta::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one Insta-specific rule; this crate only groups them.
    insta_allow_empty_glob::register_lints(sess, lint_store);
    insta_binary_snapshot_missing_extension::register_lints(sess, lint_store);
    insta_bind_to_scope_in_async::register_lints(sess, lint_store);
    insta_compact_json_snapshot::register_lints(sess, lint_store);
    insta_content_direct_match::register_lints(sess, lint_store);
    // Continue with deprecated assertions and empty-value policies.
    insta_deprecated_assert_display::register_lints(sess, lint_store);
    insta_empty_description::register_lints(sess, lint_store);
    insta_empty_filter_pattern::register_lints(sess, lint_store);
    insta_empty_input_file::register_lints(sess, lint_store);
    insta_empty_snapshot_path::register_lints(sess, lint_store);
    insta_empty_snapshot_suffix::register_lints(sess, lint_store);
    // Register path, JSON, and module-prefix policies next.
    insta_glob_parent_traversal::register_lints(sess, lint_store);
    insta_json_snapshot::register_lints(sess, lint_store);
    insta_no_module_prefix::register_lints(sess, lint_store);
    insta_noop_filter::register_lints(sess, lint_store);
    insta_redundant_content_resolve_inner::register_lints(sess, lint_store);
    // Finish with settings and loop policies.
    insta_settings_bind_future::register_lints(sess, lint_store);
    insta_settings_new::register_lints(sess, lint_store);
    insta_settings_raw_info::register_lints(sess, lint_store);
    insta_snapshot_in_loop::register_lints(sess, lint_store);
}
