#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Disallows systems in Bevy `Update`.
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
    BEVY_DISALLOW_UPDATE_SCHEDULE,
    BevyDisallowUpdateSchedule,
    "Update",
    "disallows systems in Bevy Update",
    "this project policy disallows the `Update` schedule",
    "move the system to `FixedUpdate` or another approved schedule"
}
