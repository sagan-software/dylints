# bevy-world-non-send-mut

## What it does

Checks for calls to `World::non_send_mut`.

## Why is this bad?

`World::non_send_mut` panics when the non-send value of that type is absent. A missing setup step
then stops the whole app instead of taking an error path.

## Known problems

The lint reports every call, including calls where the code already guarantees that the value
exists.

## Example

```rust
# use bevy::prelude::*;
# type WindowRegistry = std::collections::BTreeMap<usize, std::rc::Rc<()>>;
fn clear_windows(world: &mut World) {
    world.non_send_mut::<WindowRegistry>().clear();
}
```

## Use instead

```rust
# use bevy::prelude::*;
# type WindowRegistry = std::collections::BTreeMap<usize, std::rc::Rc<()>>;
fn clear_windows(world: &mut World) {
    if let Some(mut registry) = world.get_non_send_mut::<WindowRegistry>() {
        registry.clear();
    }
}
```
