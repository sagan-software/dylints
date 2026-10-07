# bevy-world-resource

## What it does

Checks for calls to `World::resource`.

## Why is this bad?

`World::resource` panics when the resource is absent. A missing setup step then stops the whole app
instead of taking an error path.

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
fn current_score(world: &World) -> u32 {
    world.resource::<Score>().0
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Resource)] struct Score(u32);
fn current_score(world: &World) -> u32 {
    world.get_resource::<Score>().map_or(0, |score| score.0)
}
```
