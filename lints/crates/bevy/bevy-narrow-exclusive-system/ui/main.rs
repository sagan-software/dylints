use bevy_app::{App, Update};
use bevy_ecs::prelude::*;

#[derive(Component)]
struct Position(f32);

fn bad(world: &mut World) {
    let mut query = world.query::<&Position>();
    let _count = query.iter(world).count();
}

fn unregistered_helper(world: &mut World) {
    let mut query = world.query::<&Position>();
    let _count = query.iter(world).count();
}

fn good(query: Query<&Position>) {
    let _sum: f32 = query.iter().map(|position| position.0).sum();
}

fn main() {
    App::new().add_systems(Update, bad);
    let _bad = bad;
    let _good = good;
    let _unregistered_helper = unregistered_helper;
}
