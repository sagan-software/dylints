#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::world::World;

struct LocalState;

fn access(world: &World) {
    let _ = world.non_send::<LocalState>();
    let _ = world.get_non_send::<LocalState>();
}

fn main() {}
