#![feature(rustc_private)]

//! Rust review lints for quantitative maintainability limits.
//!
//! This group resolves size, path, cohesion, dependency, and implementation
//! complexity measurements through one Dylint entry point. Each constituent lint
//! owns its thresholds, semantic evidence, UI fixture, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register every maintainability lint with one rustc lint store.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to each measurement pass in deterministic order, then
/// returns after the group is installed.
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = maintainability::register_lints(sess, lint_store);
/// };
/// ```
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Register quantitative and structural measurements before graph checks.
    abc_size::register_lints(sess, lint_store);
    cyclomatic_complexity::register_lints(sess, lint_store);
    many_exit_points::register_lints(sess, lint_store);
    // Register public-surface and method/type aggregation policies next.
    public_surface_size::register_lints(sess, lint_store);
    impl_method_count::register_lints(sess, lint_store);
    field_usage_cohesion::register_lints(sess, lint_store);
    source_cognitive_complexity::register_lints(sess, lint_store);
    npath_complexity::register_lints(sess, lint_store);
    type_method_complexity::register_lints(sess, lint_store);
    // Finish with module fan-out and dependency-cycle checks.
    module_fan_out::register_lints(sess, lint_store);
    module_dependency_cycle::register_lints(sess, lint_store);
}
