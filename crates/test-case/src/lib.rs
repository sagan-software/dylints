#![feature(rustc_private)]

//! Rust review lints for test-case macro misuse.
//!
//! This group resolves harness, matrix, panic-message, ignore-reason, modifier,
//! naming, and function-path patterns through one Dylint entry point. Each
//! constituent lint owns its semantic checks, UI fixture, and recommendation.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all test-case-specific lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent test-case registration function.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = test_case::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns one test-case-specific rule; this crate only groups them.
    test_case_async_without_test_harness::register_lints(sess, lint_store);
    test_case_constant_match_guard::register_lints(sess, lint_store);
    test_case_empty_description::register_lints(sess, lint_store);
    test_case_empty_ignore_reason::register_lints(sess, lint_store);
    test_case_empty_matrix::register_lints(sess, lint_store);
    // Continue with panic, ignore, and suite-size policies.
    test_case_empty_panic_message::register_lints(sess, lint_store);
    test_case_ignore_without_reason::register_lints(sess, lint_store);
    test_case_ignored_matrix::register_lints(sess, lint_store);
    test_case_inconclusive_modifier::register_lints(sess, lint_store);
    test_case_large_suite::register_lints(sess, lint_store);
    test_case_legacy_inconclusive_description::register_lints(sess, lint_store);
    // Register precision and matrix-shape policies next.
    test_case_nonpositive_almost_precision::register_lints(sess, lint_store);
    test_case_panics_without_message::register_lints(sess, lint_store);
    test_case_single_case_matrix::register_lints(sess, lint_store);
    test_case_singleton_contains_in_order::register_lints(sess, lint_store);
    test_case_singleton_matrix_dimension::register_lints(sess, lint_store);
    // Continue with naming and whitespace policies.
    test_case_unnamed_test_case::register_lints(sess, lint_store);
    test_case_whitespace_ignore_reason::register_lints(sess, lint_store);
    test_case_whitespace_panic_message::register_lints(sess, lint_store);
    test_case_wildcard_match::register_lints(sess, lint_store);
    // Finish with function-path validation.
    test_case_with_function_path::register_lints(sess, lint_store);
}
