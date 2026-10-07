# bevy-wide-query-access

## What it does

Checks for query parameters whose data contains more than five component references, or more than
four `&mut` references. References inside local custom `QueryData` types count toward the total.
For a local custom `QueryData` type, the total limit is eight references instead of five because a named type already groups related access. The `&mut` limit stays at four.

## Why is this bad?

Each reference adds access that can conflict with other systems, so wide queries limit parallel
execution. A wide query also often means one system combines several independent behaviors.

## Known problems

The lint uses fixed limits. The lint counts only `&T` and `&mut T` directly in the query data, in tuples,
and in fields of local `QueryData` types. It does not count `Option<&T>`, `Has<T>`, `Ref<T>`, or
`QueryData` types from other crates.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Position;
# #[derive(Component)] struct Velocity;
# #[derive(Component)] struct Health;
# #[derive(Component)] struct Stamina;
# #[derive(Component)] struct Target;
# #[derive(Component)] struct Team;
fn update(query: Query<(&Position, &Velocity, &Health, &Stamina, &Target, &Team)>) {
    for (position, velocity, health, stamina, target, team) in &query {
        // movement, combat, and targeting in one system
    }
}
```

## Use instead

Split the system by behavior, and give each system only the components it uses.

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Position;
# #[derive(Component)] struct Velocity;
# #[derive(Component)] struct Health;
# #[derive(Component)] struct Stamina;
# #[derive(Component)] struct Target;
# #[derive(Component)] struct Team;
fn movement(query: Query<(&Position, &Velocity)>) {
    for (position, velocity) in &query {
        // movement only
    }
}

fn combat(query: Query<(&Health, &Stamina, &Team)>) {
    for (health, stamina, team) in &query {
        // combat only
    }
}
```
