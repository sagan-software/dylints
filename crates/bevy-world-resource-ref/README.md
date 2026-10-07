# bevy-world-resource-ref

## What it does

Checks for calls to `World::resource_ref`.

## Why is this bad?

`World::resource_ref` panics when the resource is absent. A missing setup step then stops the whole
app instead of taking an error path.

## Known problems

The lint skips crates that rustc compiles as a test harness (`--test`), such as unit and
integration test builds. A panic there fails one test. The ordinary build of the same target still
checks code outside `#[cfg(test)]`.

The lint reports every call, including calls where the code already guarantees that the resource
exists.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Resource)] struct Score(u32);
fn score_changed(world: &World) -> bool {
    world.resource_ref::<Score>().is_changed()
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Resource)] struct Score(u32);
fn score_changed(world: &World) -> bool {
    world
        .get_resource_ref::<Score>()
        .is_some_and(|score| score.is_changed())
}
```
