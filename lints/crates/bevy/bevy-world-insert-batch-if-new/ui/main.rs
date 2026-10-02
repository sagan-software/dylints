#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, entity::Entity, world::World};

#[derive(Component)]
struct Marker;

fn access(world: &mut World) {
    world.insert_batch_if_new([(Entity::PLACEHOLDER, Marker)]);
    let _ = world.try_insert_batch_if_new([(Entity::PLACEHOLDER, Marker)]);
}

fn main() {}
