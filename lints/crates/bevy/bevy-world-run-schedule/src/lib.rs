#![feature(rustc_private)]
#![warn(unused_extern_crates)]

//! Checks for panicking Bevy World schedule execution.
//!
//! This Dylint library resolves Bevy World schedule calls, reports panicking
//! execution paths, and recommends handling the schedule result explicitly.
//!
//! The README defines the supported call shape and replacement. UI fixtures
//! cover triggering and non-triggering forms for safe adoption.

extern crate rustc_driver as _;
extern crate rustc_errors;
extern crate rustc_hir;

use dylint_linting as _;

#[cfg(test)]
use bevy_ecs as _;

bevy_support::declare_world_method_lint! {
    BEVY_WORLD_RUN_SCHEDULE,
    BevyWorldRunSchedule,
    Warn,
    "run_schedule",
    "try_run_schedule",
    "checks for panicking Bevy World schedule execution",
    "`World::run_schedule` panics when the schedule does not exist"
}
