# bevy-unit-in-bundle

## What it does

Checks for `()` values in the bundle passed to `spawn`, `insert`, or `insert_if_new` on `Commands`,
`World`, `EntityCommands`, `EntityWorldMut`, `RelatedSpawner`, or `RelatedSpawnerCommands`.

## Why is this bad?

A `()` in a bundle adds no component. It often comes from code that drops a call's result by mistake. Examples include a helper that returns `()` or a block that ends with a semicolon.

## Known problems

The lint does not check other bundle-taking APIs, such as `with_child` or the `children!` macro.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Player;
fn spawn_player(mut commands: Commands) {
    commands.spawn((Player, ()));
}
```

## Use instead

Remove the `()` value. To spawn an entity without components, call `spawn_empty` instead of
`spawn(())`.

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Player;
fn spawn_player(mut commands: Commands) {
    commands.spawn(Player);
}
```
