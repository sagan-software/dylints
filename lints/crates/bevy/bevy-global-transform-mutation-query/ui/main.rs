#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::system::Query;
use bevy_transform::components::{GlobalTransform, Transform};

fn bad(_: Query<&mut GlobalTransform>) {}
fn good(_: Query<&mut Transform>) {}

fn main() {}
