#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks adjacent duplicate Bevy plugin additions.
//!
//! This Dylint library resolves the relevant API or syntax, reports the
//! undesired pattern, and provides the replacement documented by its README.
//! UI fixtures cover triggering, non-triggering, and boundary forms so callers
//! can adopt the diagnostic without changing unrelated code.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;

#[cfg(test)]
use bevy_app as _;

bevy_support::declare_expression_span_lint! {
    BEVY_DUPLICATE_PLUGIN_ADDITION,
    BevyDuplicatePluginAddition,
    Warn,
    bevy_support::duplicate_plugin_addition_span,
    "checks adjacent duplicate Bevy plugin additions",
    "this unique plugin is added twice in the same chain",
    "remove the duplicate `add_plugins` call"
}
