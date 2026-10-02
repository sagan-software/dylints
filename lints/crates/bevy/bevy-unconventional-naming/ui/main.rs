#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_app::{App, Plugin};
use bevy_ecs::schedule::SystemSet;

struct Gameplay;

impl Plugin for Gameplay {
    fn build(&self, _: &mut App) {}
}

struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, _: &mut App) {}
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct Movement;

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct MovementSystems;

fn main() {}
