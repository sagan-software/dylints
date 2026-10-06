# bevy-world-entity

## What it does

Checks for calls to `World::entity`.

## Why is this bad?

`World::entity` panics when the entity does not exist. A despawned or stale `Entity` then stops the
whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the entity
exists.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Health(u32);
fn has_health(world: &World, id: Entity) -> bool {
    world.entity(id).contains::<Health>()
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Health(u32);
fn has_health(world: &World, id: Entity) -> bool {
    world
        .get_entity(id)
        .is_ok_and(|entity| entity.contains::<Health>())
}
```
