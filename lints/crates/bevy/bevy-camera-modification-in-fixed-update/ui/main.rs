#![feature(rustc_private)]

//! Systems and registration forms for camera fixed-update UI cases.

use bevy_app::{App, FixedUpdate, Update};
use bevy_camera::Camera;
use bevy_camera_modification_in_fixed_update as _;
use bevy_ecs::{query::With, schedule::IntoScheduleConfigs, system::Query};
use bevy_support as _;
use bevy_transform::components::Transform;
use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;

/// Mutates the transform of camera-filtered entities.
const fn move_camera(_: Query<'_, '_, &mut Transform, With<Camera>>) {}

/// Reads transforms of camera-filtered entities.
const fn read_camera(_: Query<'_, '_, &Transform, With<Camera>>) {}

/// Mutates transforms without a camera filter.
const fn move_other(_: Query<'_, '_, &mut Transform>) {}

/// Provides a run condition that always succeeds.
const fn always() -> bool {
    true
}

/// Registers fixed and frame-rate systems for the UI cases.
fn configure(app: &mut App) {
    {
        let _registered_app = app.add_systems(FixedUpdate, move_camera);
    }
    {
        let _registered_app = app.add_systems(
            FixedUpdate,
            (read_camera, move_camera.run_if(always)).chain(),
        );
    }
    {
        let _registered_app = app.add_systems(FixedUpdate, move_camera.after(read_camera));
    }
    {
        let _registered_app =
            app.add_systems(FixedUpdate, (move_other, read_camera.after(move_camera)));
    }
    {
        let _registered_app = app.add_systems(
            FixedUpdate,
            |_: Query<'_, '_, &mut Transform, With<Camera>>| {},
        );
    }
    {
        let _registered_app = app.add_systems(Update, move_camera);
    }
}

fn main() {
    configure(&mut App::new());
}
