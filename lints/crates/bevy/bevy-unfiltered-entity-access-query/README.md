# bevy-unfiltered-entity-access-query

## What it does

Checks for parameters of type `Query<EntityRef>` or `Query<EntityMut>` in functions that call a
typed component method on an `EntityRef` or `EntityMut`: `contains`, `get`, `get_components`,
`get_components_mut`, `get_mut`, or `get_ref`.

## Why is this bad?

`EntityRef` claims read access to every component, and `EntityMut` claims write access to every
component. The system then conflicts with all systems that write any component, even though it only
uses a fixed set.

## Known problems

The lint does not link the typed call to the query. A typed call on an `EntityRef` from another
source in the same function also triggers it. The lint does not inspect calls inside closures. The lint does not check tuple query data such as `(Entity, EntityRef)`.

## Example

```rust
fn log_positions(query: Query<EntityRef>) {
    for entity in &query {
        if let Some(position) = entity.get::<Position>() {
            info!("{}", position.0);
        }
    }
}
```

## Use instead

```rust
fn log_positions(query: Query<&Position>) {
    for position in &query {
        info!("{}", position.0);
    }
}
```
