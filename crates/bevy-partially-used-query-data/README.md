# bevy-partially-used-query-data

## What it does

Checks query parameters whose data is a local custom `QueryData` type with more than eight named fields. It warns when the function uses at most half of those fields.

## Why is this bad?

The system keeps the access of every field in the query type, including fields it never reads.
Unused `&mut` fields still stop other systems that use those components from running in parallel.

## Known problems

The lint counts fields on the derived `QueryData` item types, including inside closures. Fields with
the same name on unrelated types do not count. The lint does not count fields accessed only through
destructuring patterns.

## Example

```rust
# use bevy::ecs::{prelude::{Component, Query}, query::QueryData};
# #[derive(Component)] struct Position(pub f32);
# #[derive(Component)] struct Velocity(pub f32);
# #[derive(Component)] struct Health;
# #[derive(Component)] struct Stamina;
# #[derive(Component)] struct Hunger;
# #[derive(Component)] struct Target;
# #[derive(Component)] struct Path;
# #[derive(Component)] struct Team;
# #[derive(Component)] struct Name;
#[derive(QueryData)]
#[query_data(mutable)]
struct AgentQuery {
    position: &'static mut Position,
    velocity: &'static Velocity,
    health: &'static mut Health,
    stamina: &'static mut Stamina,
    hunger: &'static mut Hunger,
    target: &'static mut Target,
    path: &'static mut Path,
    team: &'static Team,
    name: &'static Name,
}

fn integrate(mut query: Query<AgentQuery>) {
    for mut agent in &mut query {
        agent.position.0 += agent.velocity.0;
    }
}
```

## Use instead

```rust
# use bevy::ecs::{prelude::{Component, Query}, query::QueryData};
# #[derive(Component)] struct Position(pub f32);
# #[derive(Component)] struct Velocity(pub f32);
fn integrate(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut query {
        position.0 += velocity.0;
    }
}
```
