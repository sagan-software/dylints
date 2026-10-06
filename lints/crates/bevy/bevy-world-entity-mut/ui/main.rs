#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy::ecs::{entity::Entity, world::World};

fn access(world: &mut World) {
    let id = Entity::PLACEHOLDER;
    let _ = world.entity_mut(id);
    let _ = world.get_entity_mut(id);
}

fn main() {}
