use bevy_ecs::prelude::*;

#[derive(Component)]
struct LargeAgent {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
    f: u64,
    g: u64,
    h: u64,
    i: u64,
}

#[derive(Component)]
struct Position(f32);

fn bad(query: Query<&LargeAgent, Changed<LargeAgent>>) {
    let _ = query.iter().count();
}

fn good(query: Query<&Position, Changed<Position>>) {
    let _sum: f32 = query.iter().map(|position| position.0).sum();
}

fn good_full_component(query: Query<&LargeAgent, Changed<LargeAgent>>) {
    for agent in &query {
        let _sum =
            agent.a + agent.b + agent.c + agent.d + agent.e + agent.f + agent.g + agent.h + agent.i;
    }
}

struct Mirror {
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    e: u64,
}

fn bad_same_names_elsewhere(query: Query<&LargeAgent, Changed<LargeAgent>>, mirror: Mirror) {
    let _sum = mirror.a + mirror.b + mirror.c + mirror.d + mirror.e;
    let _ = query.iter().count();
}

fn good_in_closure(query: Query<&LargeAgent, (With<Position>, Changed<LargeAgent>)>) {
    query.iter().for_each(|agent| {
        let _sum = agent.a + agent.b + agent.c + agent.d + agent.e;
    });
}

fn main() {
    let _bad = bad;
    let _good = good;
    let _good_full_component = good_full_component;
}
