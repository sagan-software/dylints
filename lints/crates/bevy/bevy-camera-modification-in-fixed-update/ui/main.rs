#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_app::{App, FixedUpdate, Update};
use bevy_camera::Camera;
use bevy_ecs::{query::With, schedule::IntoScheduleConfigs, system::Query};
use bevy_transform::components::Transform;

fn move_camera(_: Query<&mut Transform, With<Camera>>) {}

fn read_camera(_: Query<&Transform, With<Camera>>) {}

fn move_other(_: Query<&mut Transform>) {}

fn always() -> bool {
    true
}

fn configure(app: &mut App) {
    app.add_systems(FixedUpdate, move_camera);
    app.add_systems(
        FixedUpdate,
        (read_camera, move_camera.run_if(always)).chain(),
    );
    app.add_systems(FixedUpdate, (move_other, read_camera.after(move_camera)));
    app.add_systems(FixedUpdate, |_: Query<&mut Transform, With<Camera>>| {});
    app.add_systems(Update, move_camera);
}

fn main() {}
