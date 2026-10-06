use bevy::app::{App, FixedUpdate, Update};
use bevy::ecs::prelude::*;

#[derive(Component)]
struct Motion {
    x: f32,
    y: f32,
}

#[derive(Component)]
struct Heading {
    yaw: f32,
    pitch: f32,
}

#[derive(Component, Clone, Copy)]
struct Stats {
    speed: f32,
    armor: f32,
}

fn move_x(mut query: Query<&mut Motion>) {
    for mut motion in &mut query {
        motion.x += 1.0;
    }
}

fn move_y(mut query: Query<&mut Motion>) {
    for mut motion in &mut query {
        motion.y += 1.0;
    }
}

fn turn_yaw(mut query: Query<&mut Heading>) {
    query.iter_mut().for_each(|mut heading| heading.yaw += 1.0);
}

fn turn_pitch(mut query: Query<&mut Heading>) {
    query
        .iter_mut()
        .for_each(|mut heading| heading.pitch += 1.0);
}

fn boost_speed(mut query: Query<&mut Stats>) {
    for mut stats in &mut query {
        stats.speed += 1.0;
    }
}

fn reset_stats(mut query: Query<&mut Stats>) {
    for mut stats in &mut query {
        let copy = *stats;
        stats.armor = copy.armor;
    }
}

fn fixed_turn_pitch(mut query: Query<&mut Heading>) {
    for mut heading in &mut query {
        heading.pitch = 0.0;
    }
}

fn always() -> bool {
    true
}

fn main() {
    App::new()
        .add_systems(Update, (move_x, move_y))
        .add_systems(
            Update,
            (turn_yaw.run_if(always), turn_pitch, boost_speed).chain(),
        )
        .add_systems(Update, reset_stats)
        .add_systems(FixedUpdate, fixed_turn_pitch);
}
