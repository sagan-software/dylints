// run-rustfix
// rustfix-only-machine-applicable
#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_missing_clone_for_unit_component)]

use bevy_ecs::component::Component;

macro_rules! marker {
    ($name:ident) => {
        #[derive(Component)]
        struct $name;
    };
}

#[derive(Component)]
struct Missing;

/// A documented marker.
#[derive(Component)]
pub struct DocumentedMissing;

marker!(GeneratedMissing);

#[derive(Component, Clone, Copy, Default)]
struct Present;

#[derive(Component)]
struct WithField(u8);

fn main() {}
