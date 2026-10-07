# bevy-large-component-change-filter

## What it does

Checks for query parameters with a `Changed<T>` filter when `T` is a large local component and the
function uses fewer than half of `T`'s named fields.

A large component has at least eight named fields and a size over 64 bytes, as in
`bevy-large-component`.

## Why is this bad?

Bevy tracks changes per component. A write to any field of `T` marks all of `T` as changed. The filter therefore matches entities even when none of the fields this system reads have changed.

## Known problems

The lint counts field reads on the filtered component type, including inside closures. Fields with
the same name on another type do not count. The lint does not count fields accessed only through
destructuring patterns.

## Example

```rust
# use bevy::ecs::prelude::{Changed, Component, Query};
# #[derive(Component)]
# struct Agent {
#     health: u64,
#     second: u64,
#     third: u64,
#     fourth: u64,
#     fifth: u64,
#     sixth: u64,
#     seventh: u64,
#     eighth: u64,
#     ninth: u64,
# }
# macro_rules! info { ($($args:tt)*) => { println!($($args)*); }; }
// `Agent` has nine named fields, including `health`, and is over 64 bytes.
fn react_to_health(query: Query<&Agent, Changed<Agent>>) {
    for agent in &query {
        info!("health: {}", agent.health);
    }
}
```

## Use instead

Move the field into its own component and filter on that component.

```rust
# use bevy::ecs::prelude::{Changed, Component, Query};
# #[derive(Component)]
# struct Health(u64);
# macro_rules! info { ($($args:tt)*) => { println!($($args)*); }; }
fn react_to_health(query: Query<&Health, Changed<Health>>) {
    for health in &query {
        info!("health: {}", health.0);
    }
}
```
