use bevy::app::{App, Update};
use bevy::ecs::prelude::*;

#[derive(Component)]
struct Position(f32);

#[derive(Resource)]
struct Score(u32);

fn bad(world: &mut World) {
    let mut query = world.query::<&Position>();
    let _count = query.iter(world).count();
}

fn bad_resource(world: &mut World) {
    let _score = world.resource::<Score>().0;
}

fn bad_configured(world: &mut World) {
    let _score = world.get_resource::<Score>().map(|score| score.0);
}

fn good_spawn(world: &mut World) {
    world.spawn(Position(0.0));
}

fn good_escape(world: &mut World) {
    let _score = world.resource::<Score>().0;
    take(world);
}

fn good_spawn_in_closure(world: &mut World) {
    let _score = world.resource::<Score>().0;
    let mut spawn = || {
        world.spawn(Position(0.0));
    };
    spawn();
}

fn good_unnamed(_: &mut World) {}

fn take(_: &mut World) {}

fn unregistered_helper(world: &mut World) {
    let mut query = world.query::<&Position>();
    let _count = query.iter(world).count();
}

fn unregistered_query_state_get(world: &mut World) {
    let mut query = world.query::<&Position>();
    let _result = query.get(world, Entity::PLACEHOLDER);
}

fn good_other(value: &mut Position) {
    let _value = value.0;
}

fn generic_system() {}

fn generic_schedule<S: bevy::ecs::schedule::ScheduleLabel>(app: &mut App, schedule: S) {
    app.add_systems(schedule, generic_system);
}

fn closure_schedule(app: &mut App) {
    app.add_systems(Update, || {});
}

fn good(query: Query<&Position>) {
    let _sum: f32 = query.iter().map(|position| position.0).sum();
}

fn always() -> bool {
    true
}

fn main() {
    App::new()
        .add_systems(Update, bad)
        .add_systems(
            Update,
            (bad_resource, bad_configured.run_if(always)).chain(),
        )
        .add_systems(
            Update,
            (good_spawn, good_escape, good_spawn_in_closure, good_unnamed),
        );
    let _good = good;
    let _unregistered_helper = unregistered_helper;
}
