#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks for panicking mutable Bevy World non-send access.
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
use bevy_ecs as _;

bevy_support::declare_world_method_lint! {
    BEVY_WORLD_NON_SEND_MUT,
    BevyWorldNonSendMut,
    Warn,
    "non_send_mut",
    "get_non_send_mut",
    "checks for panicking mutable Bevy World non-send access",
    "`World::non_send_mut` panics when the value is absent"
}
