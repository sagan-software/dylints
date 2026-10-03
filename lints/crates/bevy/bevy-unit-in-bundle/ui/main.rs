// run-rustfix
// rustfix-only-machine-applicable
#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, entity::Entity, system::Commands, world::World};

#[derive(Component)]
struct Marker;

#[derive(Component)]
struct Health(u32);

fn unit() {}

macro_rules! with_unit {
    ($component:expr) => {
        ($component, ())
    };
}

macro_rules! spawn_unit {
    ($world:expr) => {
        $world.spawn(())
    };
}

fn spawn(world: &mut World) {
    world.spawn((Marker, ()));
    world.spawn(((), Marker));
    world.spawn((Marker, (), Health(1)));
    world.spawn(((), (), Marker));
    world.spawn((Marker, (Health(1), ())));
    world.spawn(());
    world.spawn(((), ()));
    world.spawn((Marker, unit()));
    world.spawn(with_unit!(Marker));
    spawn_unit!(world);
    world.spawn(Marker);
    world.spawn_empty();
}

fn unrelated(text: &mut String) {
    text.push_str("");
}

fn related(world: &mut World, parent: Entity) {
    world.entity_mut(parent).insert((Marker, ()));
    world.entity_mut(parent).with_children(|children| {
        children.spawn((Marker, ()));
    });
}

fn commands(mut commands: Commands, entity: Entity) {
    commands.spawn(());
    commands.entity(entity).insert((Marker, ()));
    commands.entity(entity).insert_if_new(((), Health(2)));
    let nested = (Marker, ());
    commands.spawn(nested);
    commands.entity(entity).with_children(|children| {
        children.spawn((Marker, ()));
    });
}

fn main() {}
