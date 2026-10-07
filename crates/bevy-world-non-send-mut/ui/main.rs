#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy::ecs::world::World;

struct LocalState;

fn access(world: &mut World) {
    let _ = world.non_send_mut::<LocalState>();
    let _ = world.get_non_send_mut::<LocalState>();
}

fn main() {}
