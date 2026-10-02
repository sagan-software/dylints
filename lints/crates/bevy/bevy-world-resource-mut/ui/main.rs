#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{resource::Resource, world::World};

#[derive(Resource)]
struct Score(u32);

fn access(world: &mut World) {
    let _ = world.resource_mut::<Score>();
    let _ = world.get_resource_mut::<Score>();
}

fn main() {}
