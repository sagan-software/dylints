#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, query::With, system::Query};

#[derive(Component)]
struct Marker;

#[derive(Component)]
struct Position(f32);

fn bad(_: Query<&Marker>) {}
fn good(_: Query<&Position, With<Marker>>) {}

fn main() {}
