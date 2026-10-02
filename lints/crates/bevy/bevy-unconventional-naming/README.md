# bevy-unconventional-naming

## What it does

Checks for local types that implement `Plugin` without a `Plugin` name suffix, and local types that
implement `SystemSet` without a `Systems` name suffix.

## Why is this bad?

Bevy and its ecosystem use these suffixes. Without them, readers cannot tell at a call site such as
`add_plugins(Gameplay)` or `.in_set(Movement)` what role the type has.

## Known problems

The lint does not check plugins written as functions that take `&mut App`.

## Example

```rust
struct Gameplay;

impl Plugin for Gameplay {
    fn build(&self, _app: &mut App) {}
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct Movement;
```

## Use instead

```rust
struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, _app: &mut App) {}
}

#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
struct MovementSystems;
```
