# bevy-world-resource-ref

## What it does

Checks for calls to `World::resource_ref`.

## Why is this bad?

`World::resource_ref` panics when the resource is absent. A missing setup step then stops the whole
app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the resource
exists.

## Example

```rust
fn score_changed(world: &World) -> bool {
    world.resource_ref::<Score>().is_changed()
}
```

## Use instead

```rust
fn score_changed(world: &World) -> bool {
    world
        .get_resource_ref::<Score>()
        .is_some_and(|score| score.is_changed())
}
```
