# bevy-world-non-send

## What it does

Checks for calls to `World::non_send`.

## Why is this bad?

`World::non_send` panics when the non-send value of that type is absent. A missing setup step then
stops the whole app instead of taking an error path.

## Known problems

The lint skips crates that rustc compiles as a test harness (`--test`), such as unit and
integration test builds. A panic there fails one test. The ordinary build of the same target still
checks code outside `#[cfg(test)]`.

The lint reports every call, including calls where the code already guarantees that the value
exists.

## Example

```rust
# use bevy::prelude::*;
# type WindowRegistry = std::collections::BTreeMap<usize, std::rc::Rc<()>>;
fn window_count(world: &World) -> usize {
    world.non_send::<WindowRegistry>().len()
}
```

## Use instead

```rust
# use bevy::prelude::*;
# type WindowRegistry = std::collections::BTreeMap<usize, std::rc::Rc<()>>;
fn window_count(world: &World) -> usize {
    world
        .get_non_send::<WindowRegistry>()
        .map_or(0, |registry| registry.len())
}
```
