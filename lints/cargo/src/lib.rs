#![feature(rustc_private)]

//! Rust review lints for Cargo and Rust project metadata.
//!
//! This group resolves dependency versions, key ordering, toolchain files,
//! package lint configuration, and workspace inheritance through one Dylint
//! entry point. Each constituent lint owns its semantic checks, UI fixture,
//! configuration, and recommendation for the verified metadata issue.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all Cargo and project-metadata lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent registration function, then
/// returns after the metadata group is installed.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = cargo::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    dependency_full_semver_versions::register_lints(sess, lint_store);
    dependency_key_order::register_lints(sess, lint_store);
    missing_clippy_toml::register_lints(sess, lint_store);
    missing_rust_toolchain_toml::register_lints(sess, lint_store);
    package_lints_section::register_lints(sess, lint_store);
    // Register toolchain and workspace policy passes after package-level checks.
    rust_toolchain_toml::register_lints(sess, lint_store);
    workspace_dependency_versions::register_lints(sess, lint_store);
    workspace_lints_inheritance::register_lints(sess, lint_store);
}
