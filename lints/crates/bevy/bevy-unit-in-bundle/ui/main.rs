#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, world::World};

#[derive(Component)]
struct Marker;

fn spawn(world: &mut World) {
    world.spawn((Marker, ()));
    world.spawn(Marker);
}

fn main() {}
