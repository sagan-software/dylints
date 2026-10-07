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
#![warn(bevy_mouse_displacement_times_delta)]

use bevy::ecs::{
    message::MessageReader,
    resource::Resource,
    system::{Res, ResMut},
};
use bevy::input::mouse::{AccumulatedMouseMotion, MouseMotion};
use bevy::time::{Fixed, Time};
use core::hint::black_box;
use core::marker::PhantomData;

struct Envelope<T> {
    delta: f32,
    payload: PhantomData<T>,
}

#[derive(Resource)]
struct ResourceEnvelope {
    delta: f32,
}

#[derive(Resource)]
struct CustomTime(f32);

impl CustomTime {
    const fn delta_secs(&self) -> f32 {
        self.0
    }
}

#[derive(Resource)]
struct TraitTime(f32);

trait DeltaSeconds {
    fn delta_secs(&self) -> f32;
}

impl DeltaSeconds for TraitTime {
    fn delta_secs(&self) -> f32 {
        self.0
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Positive UI cases prove resolved displacement-times-delta calls warn."
)]
fn mouse_motion(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(event.delta.x * time.delta_secs());
        black_box(f64::from(event.delta.y) * time.delta_secs_f64());
        black_box(event.delta * time.delta_secs());
        black_box(event.delta.x * time.delta().as_secs_f32());
        black_box(time.delta().as_secs_f64() * f64::from(event.delta.x));
        black_box((event.delta.x + 1.0) * time.delta_secs());
        black_box(
            (if event.delta.x > 0.0 {
                event.delta.x
            } else {
                0.0
            }) * time.delta_secs(),
        );
        black_box(({ event.delta.x }) * time.delta_secs());
        black_box(
            (match (event.delta.x > 0.0).then_some(()) {
                Some(()) => event.delta.x,
                None => 0.0,
            }) * time.delta_secs(),
        );
        black_box(event.delta.x / time.delta_secs());
        black_box((event.delta.x / time.delta_secs()) * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Unary negation still preserves mouse displacement as a value."
)]
fn mouse_motion_unary(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(-event.delta.x * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "The explicit cast preserves mouse displacement as a value."
)]
#[expect(
    clippy::cast_lossless,
    reason = "This cast exercises the lint's explicit-cast value traversal."
)]
fn mouse_motion_cast(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(event.delta.x as f64 * time.delta_secs_f64());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Subtraction preserves mouse displacement as a value."
)]
fn mouse_motion_subtraction(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box((event.delta.x - 1.0) * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Remainder arithmetic preserves mouse displacement as a value."
)]
fn mouse_motion_remainder(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box((event.delta.x % 360.0) * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Nested multiplication preserves mouse displacement as a value."
)]
fn mouse_motion_nested_multiplication(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(event.delta.x * 0.5 * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "Indexing a Bevy mouse-motion vector preserves its displacement value."
)]
fn mouse_motion_index(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(event.delta[0] * time.delta_secs());
    }
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "The reversed operand order still multiplies displacement by frame delta."
)]
fn mouse_motion_reversed(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box(time.delta_secs() * event.delta.y);
    }
}

fn accumulated_motion(motion: Res<AccumulatedMouseMotion>, time: Res<Time>) {
    black_box(motion.delta.x * time.delta_secs());
}

#[expect(
    bevy_mouse_displacement_times_delta,
    reason = "ResMut exposes the accumulated Bevy mouse displacement resource."
)]
fn accumulated_motion_mut(motion: ResMut<AccumulatedMouseMotion>, time: Res<Time>) {
    black_box(motion.delta.y * time.delta_secs());
}

fn keep_valid_units(mut motion: MessageReader<MouseMotion>, time: Res<Time>, sensitivity: f32) {
    for event in motion.read() {
        black_box(event.delta.x * sensitivity);
    }
    black_box(time.delta_secs() * sensitivity);
    black_box(1.0_f32 * time.delta_secs());
}

fn keep_lookalikes_clean(envelope: Envelope<MouseMotion>, time: Res<Time>) {
    black_box(envelope.delta * time.delta_secs());
}

fn keep_resource_lookalike_clean(envelope: Res<ResourceEnvelope>, time: Res<Time>) {
    black_box(envelope.delta * time.delta_secs());
}

fn keep_custom_time_method_clean(mut motion: MessageReader<MouseMotion>, time: Res<CustomTime>) {
    for event in motion.read() {
        black_box(event.delta.x * time.delta_secs());
    }
}

fn keep_trait_time_method_clean(mut motion: MessageReader<MouseMotion>, time: Res<TraitTime>) {
    for event in motion.read() {
        black_box(event.delta.x * time.delta_secs());
    }
}

fn keep_fixed_time_clean(mut motion: MessageReader<MouseMotion>, time: Res<Time<Fixed>>) {
    for event in motion.read() {
        black_box(event.delta.x * time.delta_secs());
        black_box(event.delta.x * time.delta().as_secs_f32());
    }
}

fn keep_non_value_uses_clean(mut motion: MessageReader<MouseMotion>, time: Res<Time>) {
    for event in motion.read() {
        black_box((if event.delta.x > 0.0 { 1.0 } else { 0.5 }) * time.delta_secs());
        black_box(
            (match (event.delta.x > 0.0).then_some(()) {
                Some(()) => 1.0,
                None => 0.5,
            }) * time.delta_secs(),
        );
        black_box(
            ({
                black_box(event.delta.x);
                1.0
            }) * time.delta_secs(),
        );
        black_box((f32::from(u8::from(event.delta.x > 0.0))) * time.delta_secs());
    }
}

fn main() {}
