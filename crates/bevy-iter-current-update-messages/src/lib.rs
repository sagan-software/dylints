#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks direct iteration over only current-update Bevy messages.
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

bevy_support::declare_expression_span_lint! {
    BEVY_ITER_CURRENT_UPDATE_MESSAGES,
    BevyIterCurrentUpdateMessages,
    Warn,
    bevy_support::iter_current_update_messages_span,
    "checks direct iteration over only current-update Bevy messages",
    "`iter_current_update_messages` has a narrow update-window contract",
    "use `MessageReader<M>` and call `read`"
}
