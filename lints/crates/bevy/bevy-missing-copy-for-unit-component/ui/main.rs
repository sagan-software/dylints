#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_missing_copy_for_unit_component)]

use bevy_ecs::component::Component;

#[derive(Component)]
struct Missing;

#[derive(Component, Clone, Copy)]
struct Present;

fn main() {}
