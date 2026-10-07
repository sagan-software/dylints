#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy::ecs::{component::Component, query::With, system::Query};

#[derive(Component)]
struct Marker;

#[derive(Component)]
struct Position(f32);

fn bad(_: Query<&Marker>) {}
fn bad_tuple(_: Query<(&Position, &mut Marker)>) {}

fn bad_closure() {
    let _closure = |_: Query<&Marker>| {};
}

fn good(_: Query<&Position, With<Marker>>) {}
fn good_option(_: Query<Option<&Marker>>) {}

trait MarkerSystem {
    fn run(&self, query: Query<&Marker>);
}

struct Runner;

impl MarkerSystem for Runner {
    fn run(&self, _: Query<&Marker>) {}
}

fn main() {}
