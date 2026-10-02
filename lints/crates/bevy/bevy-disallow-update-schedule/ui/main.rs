#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_disallow_update_schedule)]

use bevy_app::{App, FixedUpdate, Update};

fn tick() {}

fn configure(app: &mut App) {
    app.add_systems(Update, tick);
    app.add_systems(FixedUpdate, tick);
}

fn main() {}
