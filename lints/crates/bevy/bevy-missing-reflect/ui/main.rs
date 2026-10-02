#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_missing_reflect)]

use bevy_ecs::component::Component;
use bevy_reflect::Reflect;

#[derive(Component)]
struct Missing;

#[derive(Component, Reflect)]
struct Present;

fn main() {}
