#![feature(rustc_private)]

//! UI cases for derived Bevy query-data field ownership.

use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use bevy_partially_used_query_data as _;
use bevy_support as _;
use dylint_linting as _;
use dylint_support as _;
use dylint_testing as _;

/// Component marker for the first query field.
#[derive(Component)]
struct A(
    /// Scalar payload for the first field.
    f32,
);
/// Component marker for the second query field.
#[derive(Component)]
struct B;
/// Component marker for the third query field.
#[derive(Component)]
struct C;
/// Component marker for the fourth query field.
#[derive(Component)]
struct D;
/// Component marker for the fifth query field.
#[derive(Component)]
struct E;
/// Component marker for the sixth query field.
#[derive(Component)]
struct F;
/// Component marker for the seventh query field.
#[derive(Component)]
struct G;
/// Component marker for the eighth query field.
#[derive(Component)]
struct H;
/// Component marker for the ninth query field.
#[derive(Component)]
struct I;

/// Nine-field immutable query used by the field-count cases.
#[derive(QueryData)]
struct AgentQuery {
    /// First component field.
    a: &'static A,
    /// Second component field.
    b: &'static B,
    /// Third component field.
    c: &'static C,
    /// Fourth component field.
    d: &'static D,
    /// Fifth component field.
    e: &'static E,
    /// Sixth component field.
    f: &'static F,
    /// Seventh component field.
    g: &'static G,
    /// Eighth component field.
    h: &'static H,
    /// Ninth component field.
    i: &'static I,
}

/// Uses one query field and triggers the partial-use warning.
fn bad(query: Query<'_, '_, AgentQuery>) {
    for agent in &query {
        let _ = std::hint::black_box(agent.a.0);
    }
}

/// Unrelated fields share names with the unused query fields.
struct Unrelated {
    /// Unrelated field `e`.
    e: u8,
    /// Unrelated field `f`.
    f: u8,
    /// Unrelated field `g`.
    g: u8,
    /// Unrelated field `h`.
    h: u8,
    /// Unrelated field `i`.
    i: u8,
}

/// Reads unrelated same-name fields and four actual query fields.
fn bad_same_names_elsewhere(query: Query<'_, '_, AgentQuery>, other: &Unrelated) {
    let _ = std::hint::black_box((other.e, other.f, other.g, other.h, other.i));
    for agent in &query {
        let _ = std::hint::black_box((agent.a.0, agent.b, agent.c, agent.d));
    }
}

/// Reads five query fields inside a closure.
fn good_in_closure(query: Query<'_, '_, AgentQuery>) {
    query.iter().for_each(|agent| {
        let _ = std::hint::black_box((agent.a, agent.b, agent.c, agent.d, agent.e));
    });
}

/// Nine-field mutable query used by mutable-item cases.
#[derive(QueryData)]
#[query_data(mutable)]
struct MutableAgentQuery {
    /// First mutable component field.
    a: &'static mut A,
    /// Second component field.
    b: &'static B,
    /// Third component field.
    c: &'static C,
    /// Fourth component field.
    d: &'static D,
    /// Fifth component field.
    e: &'static E,
    /// Sixth component field.
    f: &'static F,
    /// Seventh component field.
    g: &'static G,
    /// Eighth component field.
    h: &'static H,
    /// Ninth component field.
    i: &'static I,
}

/// Eight-field immutable query used as a quiet boundary control.
#[derive(QueryData)]
struct EightFieldQuery {
    /// First component field.
    a: &'static A,
    /// Second component field.
    b: &'static B,
    /// Third component field.
    c: &'static C,
    /// Fourth component field.
    d: &'static D,
    /// Fifth component field.
    e: &'static E,
    /// Sixth component field.
    f: &'static F,
    /// Seventh component field.
    g: &'static G,
    /// Eighth component field.
    h: &'static H,
}

/// Reads fields from the read-only query item.
#[expect(
    clippy::needless_pass_by_value,
    reason = "Bevy system query parameters are passed by value."
)]
fn good_read_only_item(query: Query<'_, '_, MutableAgentQuery>) {
    for agent in query.iter() {
        let _ = std::hint::black_box((agent.a, agent.b, agent.c, agent.d, agent.e));
    }
}

/// Mutates one field through a mutable query item.
fn bad_mutable(mut query: Query<'_, '_, MutableAgentQuery>) {
    for mut agent in &mut query {
        agent.a.0 += 1.0;
    }
}

/// Uses an eight-field query that exceeds the warning threshold.
fn good_small(query: Query<'_, '_, EightFieldQuery>) {
    let _ = query.iter().next();
}

/// Reads a direct component query instead of derived query data.
fn good(query: Query<'_, '_, &A>) {
    for a in &query {
        let _ = std::hint::black_box(a.0);
    }
}

/// Keeps the UI functions and unrelated control type reachable.
fn main() {
    let _ = std::hint::black_box((
        bad,
        bad_same_names_elsewhere,
        good_in_closure,
        good_read_only_item,
        bad_mutable,
        good_small,
        good,
    ));
    let _ = std::hint::black_box(Unrelated {
        e: 0,
        f: 0,
        g: 0,
        h: 0,
        i: 0,
    });
}
