# bevy-world-resource-mut

## What it does

Checks for calls to `World::resource_mut`.

## Why is this bad?

`World::resource_mut` panics when the resource is absent. A missing setup step then stops the whole
app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the resource
exists.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Resource)] struct Score(u32);
fn add_point(world: &mut World) {
    world.resource_mut::<Score>().0 += 1;
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Resource)] struct Score(u32);
fn add_point(world: &mut World) {
    if let Some(mut score) = world.get_resource_mut::<Score>() {
        score.0 += 1;
    }
}
```
