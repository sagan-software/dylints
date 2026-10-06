#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_missing_reflect)]

use bevy::ecs::component::Component;
use bevy::reflect::Reflect;

#[derive(Component)]
struct Missing;

#[derive(Component, Reflect)]
struct Present;

fn main() {}
