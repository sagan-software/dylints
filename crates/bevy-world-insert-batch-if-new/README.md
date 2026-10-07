# bevy-world-insert-batch-if-new

## What it does

Checks for calls to `World::insert_batch_if_new`.

## Why is this bad?

`World::insert_batch_if_new` panics when any entity in the batch does not exist. One despawned
entity then stops the whole app instead of taking an error path.

## Known problems

The lint skips crates that rustc compiles as a test harness (`--test`), such as unit and
integration test builds. A panic there fails one test. The ordinary build of the same target still
checks code outside `#[cfg(test)]`.

The lint reports every call, including calls where the code already guarantees that every entity
exists.

## Example

```rust
# use bevy::prelude::*;
# use bevy::ecs::error::BevyError;
# #[derive(Component)] struct Marker;
fn mark_all(world: &mut World, ids: Vec<Entity>) {
    world.insert_batch_if_new(ids.into_iter().map(|id| (id, Marker)));
}
```

## Use instead

```rust
# use bevy::prelude::*;
# use bevy::ecs::error::BevyError;
# #[derive(Component)] struct Marker;
fn mark_all(world: &mut World, ids: Vec<Entity>) -> Result<(), BevyError> {
    world.try_insert_batch_if_new(ids.into_iter().map(|id| (id, Marker)))?;
    Ok(())
}
```
