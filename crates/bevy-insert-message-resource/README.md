# bevy-insert-message-resource

## What it does

Checks for `App::insert_resource` and `App::init_resource` calls that add a `Messages<M>` resource.

## Why is this bad?

`App::add_message` adds the `Messages<M>` resource and the system that clears old messages each
frame. A manually added resource lacks that system, so no system drops old messages and the buffer grows without bound.

## Known problems

The lint does not check `World::insert_resource`, `World::init_resource`, or `Commands`.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Message)] struct Ping;
fn build(app: &mut App) {
    app.init_resource::<Messages<Ping>>();
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Message)] struct Ping;
fn build(app: &mut App) {
    app.add_message::<Ping>();
}
```
