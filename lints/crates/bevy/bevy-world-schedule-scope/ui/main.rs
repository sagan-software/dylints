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
    world.schedule_scope(Gameplay, |_, _| {});
    let _ = world.try_schedule_scope(Gameplay, |_, _| {});
}

fn main() {}
