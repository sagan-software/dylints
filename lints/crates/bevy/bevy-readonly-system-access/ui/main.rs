#![allow(
    dead_code,
    elided_lifetimes_in_paths,
    let_underscore_drop,
    missing_docs,
    unknown_lints,
    unused_results
)]

use bevy_ecs::{component::Component, entity::Entity, system::Query};

#[derive(Component)]
struct Position(f32);

#[derive(Component)]
struct Velocity(f32);

type PositionQuery<'w, 's> = Query<'w, 's, &'static mut Position>;

fn bad(query: Query<&mut Position>) {
    let _count = query.iter().count();
}

fn bad_tuple(query: Query<(&mut Position, &'static mut Velocity)>, entity: Entity) {
    let _found = query.get(entity).is_ok();
}

fn bad_reference(query: &Query<&mut Position>) {
    let _empty = query.is_empty();
}

fn bad_alias(query: PositionQuery) {
    let _found = query.single().is_ok();
}

fn bad_in_closure(query: Query<&mut Position>) {
    let total = || query.iter().map(|position| position.0).sum::<f32>();
    let _total = total();
}

fn good(mut query: Query<&mut Position>) {
    for mut position in &mut query {
        position.0 += 1.0;
    }
}

fn good_mutation_in_closure(mut query: Query<&mut Position>) {
    let _count = query.iter().count();
    let mut advance = || {
        for mut position in &mut query {
            position.0 += 1.0;
        }
    };
    advance();
}

fn good_shared(query: Query<&Position>) {
    let _count = query.iter().count();
}

fn good_unused(_: Query<&mut Position>) {}

trait Inspect {
    fn inspect(&self, query: Query<&mut Position>);
}

struct Inspector;

impl Inspect for Inspector {
    fn inspect(&self, query: Query<&mut Position>) {
        let _count = query.iter().count();
    }
}

fn main() {}
