#![feature(rustc_private)]

//! UI cases for component-field ownership and closure traversal.

use bevy::ecs::prelude::*;
use bevy_large_component_change_filter as _;
use bevy_support as _;
use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;

/// Synthetic component with nine fields for change-filter cases.
#[derive(Component)]
struct LargeAgent {
    /// First scalar field.
    a: u64,
    /// Second scalar field.
    b: u64,
    /// Third scalar field.
    c: u64,
    /// Fourth scalar field.
    d: u64,
    /// Fifth scalar field.
    e: u64,
    /// Sixth scalar field.
    f: u64,
    /// Seventh scalar field.
    g: u64,
    /// Eighth scalar field.
    h: u64,
    /// Ninth scalar field.
    i: u64,
}

/// Small component used by the non-large control.
#[derive(Component)]
struct Position(
    /// Position coordinate.
    f32,
);

/// Leaves the large component query values unused.
fn bad(query: Query<'_, '_, &LargeAgent, Changed<LargeAgent>>) {
    let _ = query.iter().count();
}

/// Reads the small component in the quiet control.
fn good(query: Query<'_, '_, &Position, Changed<Position>>) {
    let _sum: f32 = query.iter().map(|position| position.0).sum();
}

/// Reads every large-component field in the quiet control.
fn good_full_component(query: Query<'_, '_, &LargeAgent, Changed<LargeAgent>>) {
    for agent in &query {
        let _ = std::hint::black_box(
            agent.a + agent.b + agent.c + agent.d + agent.e + agent.f + agent.g + agent.h + agent.i,
        );
    }
}

/// Unrelated component fields share names with `LargeAgent` fields.
struct Mirror {
    /// Unrelated mirror field `e`.
    e: u64,
    /// Unrelated mirror field `f`.
    f: u64,
    /// Unrelated mirror field `g`.
    g: u64,
    /// Unrelated mirror field `h`.
    h: u64,
    /// Unrelated mirror field `i`.
    i: u64,
}

/// Reads unrelated same-named fields and four queried fields.
fn bad_same_names_elsewhere(
    query: Query<'_, '_, &LargeAgent, Changed<LargeAgent>>,
    mirror: &Mirror,
) {
    let _ = std::hint::black_box(mirror.e + mirror.f + mirror.g + mirror.h + mirror.i);
    for agent in &query {
        let _ = std::hint::black_box(agent.a + agent.b + agent.c + agent.d);
    }
}

/// Reads five fields in a closure, preventing the warning.
fn good_in_closure(query: Query<'_, '_, &LargeAgent, (With<Position>, Changed<LargeAgent>)>) {
    query.iter().for_each(|agent| {
        let _ = std::hint::black_box(agent.a + agent.b + agent.c + agent.d + agent.e);
    });
}

/// Keeps UI functions and the unrelated component reachable.
fn main() {
    let _ = std::hint::black_box((
        bad,
        good,
        good_full_component,
        bad_same_names_elsewhere,
        good_in_closure,
    ));
    let _ = std::hint::black_box(Mirror {
        e: 0,
        f: 0,
        g: 0,
        h: 0,
        i: 0,
    });
}
