# bevy-partially-used-query-data

## What it does

Checks query parameters whose data is a local custom `QueryData` type with more than eight named fields. It warns when the function uses at most half of those fields.

## Why is this bad?

The system keeps the access of every field in the query type, including fields it never reads.
Unused `&mut` fields still stop other systems that use those components from running in parallel.

## Known problems

The lint matches fields by name anywhere in the function body, so a field with the same name on another type counts as a use. The lint does not count field accesses inside closures or in destructuring patterns.

## Example

```rust
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
fn integrate(mut query: Query<(&mut Position, &Velocity)>) {
    for (mut position, velocity) in &mut query {
        position.0 += velocity.0;
    }
}
```
