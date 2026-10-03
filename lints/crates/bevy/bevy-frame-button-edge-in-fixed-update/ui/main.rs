#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::needless_pass_by_value,
    unknown_lints,
    unused_crate_dependencies,
    unused_results,
    reason = "Compiletest fixtures model Bevy system signatures and lint inputs."
)]
#![deny(unfulfilled_lint_expectations)]
#![warn(bevy_frame_button_edge_in_fixed_update)]

use bevy_app::{App, FixedUpdate, Update};
use bevy_ecs::{
    resource::Resource,
    system::{Res, ResMut},
};
use bevy_input::{ButtonInput, keyboard::KeyCode};

mod custom_schedule {
    use bevy_ecs::schedule::ScheduleLabel;

    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct FixedUpdate;
}

fn pressed(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {}
}

fn tuple_pressed((keys,): (Res<ButtonInput<KeyCode>>,)) {
    if keys.just_pressed(KeyCode::Space) {}
}

fn released(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_released(KeyCode::Space) {}
}

fn held(keys: Res<ButtonInput<KeyCode>>) {
    if keys.pressed(KeyCode::Space) {}
}

#[derive(Resource, Default)]
struct BufferedInput {
    jump: bool,
}

fn buffered(input: Res<BufferedInput>) {
    if input.jump {}
}

fn explicitly_cleared(mut keys: ResMut<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {
        keys.clear_just_pressed(KeyCode::Space);
    }
}

fn update_only(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::Space) {}
}

fn tuple_update_only((keys,): (Res<ButtonInput<KeyCode>>,)) {
    if keys.just_pressed(KeyCode::Space) {}
}

fn tuple_custom_schedule((keys,): (Res<ButtonInput<KeyCode>>,)) {
    if keys.just_pressed(KeyCode::Space) {}
}

const fn helper_without_system_params(_flag: bool) {}

fn keep_uninvoked<F>(_deferred: F) {}

fn deferred(keys: Res<ButtonInput<KeyCode>>) {
    let deferred = || keys.just_pressed(KeyCode::Space);
    keep_uninvoked(deferred);
}

fn configure(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (pressed, released, held, buffered, explicitly_cleared),
    );
    app.add_systems(FixedUpdate, tuple_pressed);
    app.add_systems(Update, update_only);
    app.add_systems(Update, tuple_update_only);
    app.add_systems(custom_schedule::FixedUpdate, pressed);
    app.add_systems(custom_schedule::FixedUpdate, tuple_custom_schedule);
    app.add_systems(FixedUpdate, deferred);
    app.add_systems(
        FixedUpdate,
        |keys: Res<ButtonInput<KeyCode>>| {
            if keys.just_pressed(KeyCode::Space) {}
        },
    );
}

fn main() {}
