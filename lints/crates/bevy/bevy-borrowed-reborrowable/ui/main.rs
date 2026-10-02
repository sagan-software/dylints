#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, system::Query};

#[derive(Component)]
struct Marker;

fn bad(_: &mut Query<&Marker>) {}
fn good(_: Query<&Marker>) {}

fn main() {}
