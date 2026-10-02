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

struct Unrelated {
    b: u8,
    c: u8,
    d: u8,
    e: u8,
}

fn bad_same_names_elsewhere(query: Query<AgentQuery>, other: Unrelated) {
    let _sum = other.b + other.c + other.d + other.e;
    for agent in &query {
        let _x = agent.a.0;
    }
}

fn good_in_closure(query: Query<AgentQuery>) {
    query.iter().for_each(|agent| {
        let _all = (agent.a, agent.b, agent.c, agent.d, agent.e);
    });
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MutableAgentQuery {
    a: &'static mut A,
    b: &'static B,
    c: &'static C,
    d: &'static D,
    e: &'static E,
    f: &'static F,
    g: &'static G,
    h: &'static H,
    i: &'static I,
}

fn good_read_only_item(query: Query<MutableAgentQuery>) {
    for agent in query.iter() {
        let _all = (agent.a, agent.b, agent.c, agent.d, agent.e);
    }
}

fn bad_mutable(mut query: Query<MutableAgentQuery>) {
    for mut agent in &mut query {
        agent.a.0 += 1.0;
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
