#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{schedule::ScheduleLabel, world::World};

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Gameplay;

fn access(world: &mut World) {
    world.run_schedule(Gameplay);
    let _ = world.try_run_schedule(Gameplay);
}

fn main() {}
