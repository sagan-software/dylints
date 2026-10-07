#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Disallows systems in Bevy `FixedUpdate`.
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

bevy_support::declare_disallowed_schedule_lint! {
    BEVY_DISALLOW_FIXED_UPDATE_SCHEDULE,
    BevyDisallowFixedUpdateSchedule,
    "FixedUpdate",
    "disallows systems in Bevy FixedUpdate",
    "this project policy disallows the `FixedUpdate` schedule",
    "move the system to `Update` or another approved schedule"
}
