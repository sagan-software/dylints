use bevy::ecs::prelude::*;
use bevy::ecs::query::QueryData;

#[derive(Component)]
struct A;
#[derive(Component)]
struct B;
#[derive(Component)]
struct C;
#[derive(Component)]
struct D;
#[derive(Component)]
struct E;
#[derive(Component)]
struct F;
#[derive(Component)]
struct G;
#[derive(Component)]
struct H;
#[derive(Component)]
struct I;

#[derive(QueryData)]
struct WideQuery {
    a: &'static A,
    b: &'static B,
    c: &'static C,
    d: &'static D,
    e: &'static E,
    f: &'static F,
    g: &'static G,
    h: &'static H,
    i: &'static I,
}

#[derive(QueryData)]
struct EightQuery {
    a: &'static A,
    b: &'static B,
    c: &'static C,
    d: &'static D,
    e: &'static E,
    f: &'static F,
    g: &'static G,
    h: &'static H,
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MutableQuery {
    a: &'static mut A,
    b: &'static mut B,
    c: &'static mut C,
    d: &'static mut D,
    e: &'static mut E,
}

fn bad(query: Query<(&A, &B, &C, &D, &E, &F)>) {
    let _ = query.iter().count();
}

fn good(query: Query<(&A, &B, &C, &D, &E)>) {
    let _ = query.iter().count();
}

fn bad_custom(query: Query<WideQuery>) {
    let _ = query.iter().count();
}

fn good_custom(query: Query<EightQuery>) {
    let _ = query.iter().count();
}

fn good_empty(entity_query: Query<Entity>, unit_query: Query<()>) {
    let _ = entity_query.iter().count();
    let _ = unit_query.iter().count();
}

fn bad_custom_mutable(mut query: Query<MutableQuery>) {
    let _ = query.iter_mut().count();
}

fn main() {
    let _bad = bad;
    let _good = good;
    let _bad_custom = bad_custom;
    let _good_custom = good_custom;
    let _good_empty = good_empty;
    let _bad_custom_mutable = bad_custom_mutable;
}
