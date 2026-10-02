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
struct Position(f32);

fn bad(query: Query<&mut Position>) {
    let _count = query.iter().count();
}

fn good(mut query: Query<&mut Position>) {
    for mut position in &mut query {
        position.0 += 1.0;
    }
}

fn main() {}
