#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! All Sagan lint groups through one standard Dylint library.
//!
//! This optional aggregate registers the nine category libraries in one compiler
//! store. The crate-specific category owns its nested registrations. Discover
//! this library independently: loading its constituent groups alongside it
//! registers duplicate lint names. Consumers can select individual categories
//! instead when they need a smaller policy scope.

extern crate rustc_lint;
extern crate rustc_session;

#[cfg(not(feature = "rlib"))]
dylint_linting::dylint_library!();

#[cfg(feature = "rlib")]
use dylint_linting as _;

/// Register each group once, including the nested crate-specific lints.
///
/// The caller supplies the active compiler session and mutable lint store.
/// Do not register constituent groups separately in that same store.
///
/// # Examples
///
/// ```rust
/// # #![feature(rustc_private)]
/// let register = sagan_lints::register_lints;
/// let _ = register;
/// ```
#[cfg_attr(not(feature = "rlib"), unsafe(no_mangle))]
pub fn register_lints(session: &rustc_session::Session, store: &mut rustc_lint::LintStore) {
    // General Cargo, control-flow and correctness checks share one compiler store.
    cargo_lints::register_lints(session, store);
    complexity_lints::register_lints(session, store);
    correctness_lints::register_lints(session, store);
    // The crates group owns its nested registrations; no child group is loaded twice.
    crates_lints::register_lints(session, store);
    maintainability_lints::register_lints(session, store);
    perf_lints::register_lints(session, store);
    // Register the remaining policy and source-style groups after the semantic groups.
    restriction_lints::register_lints(session, store);
    style_lints::register_lints(session, store);
    suspicious_lints::register_lints(session, store);
}

/// Load the aggregate through the compiler so duplicate registration fails the UI test.
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
