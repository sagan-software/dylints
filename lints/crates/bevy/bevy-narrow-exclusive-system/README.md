# bevy-narrow-exclusive-system

## What it does

Checks for systems that take `&mut World` and use it only for typed query, resource, or entity
access. The checked `World` methods are `entities`, `get`, `get_mut`, `get_resource`,
`get_resource_mut`, `query`, `query_filtered`, `resource`, and `resource_mut`, plus `QueryState`
methods that take the world.

## Why is this bad?

An exclusive system blocks every other system while it runs. Normal system parameters declare only
the access the system needs, so Bevy can run it in parallel with other systems.

## Known problems

The lint only checks free functions passed to `App::add_systems` as a plain path or in a tuple of
paths. It skips methods, closures, and functions that pass the world to other code.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Position;
fn count_positions(world: &mut World) {
    let mut query = world.query::<&Position>();
    let count = query.iter(world).count();
    info!("{count} positions");
}

fn build(app: &mut App) {
    app.add_systems(Update, count_positions);
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Position;
fn count_positions(query: Query<&Position>) {
    let count = query.iter().count();
    info!("{count} positions");
}
```
