#![feature(rustc_private)]

//! Rust review lints for unnecessarily complex code.
//!
//! This group resolves collection, iterator, control-flow, parsing, and
//! abstraction patterns through one Dylint entry point. Each constituent lint
//! owns its semantic checks, UI fixture, configuration, and recommendation for
//! the verified complexity issue.
//! The group preserves per-lint configuration and diagnostic ownership.

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

extern crate rustc_lint;
extern crate rustc_session;

/// Register all complexity lints in deterministic order.
///
/// Dylint calls this function once per compilation. It forwards the compiler
/// session and lint store to every constituent registration function, then
/// returns after the complexity group is installed.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let _call = |sess, lint_store| {
///     let _ = complexity::register_lints(sess, lint_store);
/// };
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // Each constituent crate owns its lint pass; this crate only defines the group.
    collect_return::register_lints(sess, lint_store);
    complicated_conditional::register_lints(sess, lint_store);
    consecutive_iterator_loops::register_lints(sess, lint_store);
    contains_key_before_any::register_lints(sess, lint_store);
    // Register control-flow simplifications before iterator-loop families.
    let_some_return_err::register_lints(sess, lint_store);
    manual_adjacent_window_loop::register_lints(sess, lint_store);
    manual_arg_parsing::register_lints(sess, lint_store);
    manual_entry_update::register_lints(sess, lint_store);
    manual_extend_loop::register_lints(sess, lint_store);
    // Keep fallible and filtering loop transformations adjacent in the group.
    manual_fallible_collect_loop::register_lints(sess, lint_store);
    manual_filter_for_each_loop::register_lints(sess, lint_store);
    manual_filter_map_for_each_loop::register_lints(sess, lint_store);
    manual_iterator_loop::register_lints(sess, lint_store);
    manual_option_take_if::register_lints(sess, lint_store);
    // Register collection and passthrough transformations after general iterator checks.
    manual_partition_loop::register_lints(sess, lint_store);
    manual_passthrough_inspect::register_lints(sess, lint_store);
    manual_try_for_each_loop::register_lints(sess, lint_store);
    manual_unzip_loop::register_lints(sess, lint_store);
    one_use_predicate_binding::register_lints(sess, lint_store);
    // Finish with local abstraction and return-shape checks.
    one_use_private_helper::register_lints(sess, lint_store);
    trivial_public_constructor::register_lints(sess, lint_store);
    unnecessary_map_err::register_lints(sess, lint_store);
}
