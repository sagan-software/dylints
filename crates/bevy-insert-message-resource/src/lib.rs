#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks manual insertion of Bevy Messages resources.
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
use bevy as _;
#[cfg(test)]
use bevy as _;

bevy_support::declare_expression_span_lint! {
    BEVY_INSERT_MESSAGE_RESOURCE,
    BevyInsertMessageResource,
    Warn,
    bevy_support::inserted_message_resource_span,
    "checks manual insertion of Bevy Messages resources",
    "manually inserting `Messages` omits Bevy's message update system",
    "use `App::add_message::<M>()`"
}
