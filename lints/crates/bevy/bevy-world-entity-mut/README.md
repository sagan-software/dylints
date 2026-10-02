# bevy-world-entity-mut

## What it does

Checks for calls to `World::entity_mut`.

## Why is this bad?

`World::entity_mut` panics when the entity does not exist. A despawned or stale `Entity` then stops
the whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the entity
exists.

## Example

```rust
fn heal(world: &mut World, id: Entity) {
    world.entity_mut(id).insert(Health(100));
}
```

## Use instead

```rust
fn heal(world: &mut World, id: Entity) {
    if let Ok(mut entity) = world.get_entity_mut(id) {
        entity.insert(Health(100));
    }
}
```
