use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;

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

fn bad(query: Query<(&A, &B, &C, &D, &E, &F)>) {
    let _ = query.iter().count();
}

fn good(query: Query<(&A, &B, &C, &D, &E)>) {
    let _ = query.iter().count();
}

fn bad_custom(query: Query<WideQuery>) {
    let _ = query.iter().count();
}

fn main() {
    let _bad = bad;
    let _good = good;
    let _bad_custom = bad_custom;
}
