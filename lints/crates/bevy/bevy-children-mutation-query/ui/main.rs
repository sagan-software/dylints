#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{
    hierarchy::{ChildOf, Children},
    system::Query,
};

fn bad(_: Query<&mut Children>) {}
fn good(_: Query<&ChildOf>) {}

fn main() {}
