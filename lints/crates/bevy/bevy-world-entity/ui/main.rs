#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{entity::Entity, world::World};

fn access(world: &World) {
    let id = Entity::PLACEHOLDER;
    let _ = world.entity(id);
    let _ = world.get_entity(id);
}

fn main() {}
