# bevy-missing-clone-for-unit-component

## What it does

Checks for unit structs, such as `struct Player;`, that implement `Component` but not `Clone`.

## Why is this bad?

Bevy's entity cloning copies a component through `Clone` when it is implemented. Without `Clone`,
the marker depends on the world's default clone handler, which can skip it, so a cloned entity can
lose the marker. A unit struct has no state, so `Clone` costs nothing.

## Known problems

None known.

## Example

```rust
#[derive(Component)]
struct Player;
```

## Use instead

```rust
#[derive(Component, Clone)]
struct Player;
```
