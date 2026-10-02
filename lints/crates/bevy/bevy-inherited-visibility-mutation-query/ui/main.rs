#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_camera::visibility::{InheritedVisibility, Visibility};
use bevy_ecs::system::Query;

fn bad(_: Query<&mut InheritedVisibility>) {}
fn good(_: Query<&mut Visibility>) {}

fn main() {}
