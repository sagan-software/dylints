#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]
#![warn(bevy_disallow_fixed_update_schedule)]

use bevy::app::{App, FixedUpdate, Update};

fn animate() {}

fn configure(app: &mut App) {
    app.add_systems(FixedUpdate, animate);
    app.add_systems(Update, animate);
}

fn main() {}
