# bevy-world-resource

## What it does

Checks for calls to `World::resource`.

## Why is this bad?

`World::resource` panics when the resource is absent. A missing setup step then stops the whole app
instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the resource
exists.

## Example

```rust
fn current_score(world: &World) -> u32 {
    world.resource::<Score>().0
}
```

## Use instead

```rust
fn current_score(world: &World) -> u32 {
    world.get_resource::<Score>().map_or(0, |score| score.0)
}
```
