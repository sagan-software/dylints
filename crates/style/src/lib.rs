#![feature(rustc_private)]

//! Rust review lints for idiomatic style.
//!
//! This group resolves conversion traits, naming, imports, manual implementations,
//! module layout, and documentation-comment patterns through one Dylint entry
//! point. Each constituent lint owns its semantic checks, UI fixture,
//! configuration, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all style lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent registration function, then
/// returns after the style group is installed.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = style::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    interleaved_doc_attributes::register_lints(sess, lint_store);
    ad_hoc_as_ref::register_lints(sess, lint_store);
    ad_hoc_borrow::register_lints(sess, lint_store);
    ad_hoc_default::register_lints(sess, lint_store);
    ad_hoc_display::register_lints(sess, lint_store);
    ad_hoc_from::register_lints(sess, lint_store);
    // Keep standard conversion-trait suggestions together.
    ad_hoc_from_str::register_lints(sess, lint_store);
    ad_hoc_into_iterator::register_lints(sess, lint_store);
    ad_hoc_iterator::register_lints(sess, lint_store);
    ad_hoc_try_from::register_lints(sess, lint_store);
    bool_name_prefix::register_lints(sess, lint_store);
    // Register documentation and import-style checks after API naming checks.
    doc_attr_comment::register_lints(sess, lint_store);
    internal_import_self::register_lints(sess, lint_store);
    manual_debug_impl::register_lints(sess, lint_store);
    manual_default_impl::register_lints(sess, lint_store);
    manual_error_impl::register_lints(sess, lint_store);
    // Finish with module-layout and documentation-comment checks.
    module_type_first::register_lints(sess, lint_store);
    rumdl_doc_comments::register_lints(sess, lint_store);
    top_level_item_line_break::register_lints(sess, lint_store);
}
