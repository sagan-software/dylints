use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;

#[derive(Component)]
struct A(f32);
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
struct AgentQuery {
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

fn bad(query: Query<AgentQuery>) {
    for agent in &query {
        let _x = agent.a.0;
    }
}

fn good(query: Query<&A>) {
    for a in &query {
        let _x = a.0;
    }
}

fn main() {
    let _bad = bad;
    let _good = good;
}
