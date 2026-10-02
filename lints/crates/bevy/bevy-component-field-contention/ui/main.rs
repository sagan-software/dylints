use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

#[derive(Component)]
struct Motion {
    x: f32,
    y: f32,
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

fn main() {
    App::new().add_systems(Update, (move_x, move_y));
}
