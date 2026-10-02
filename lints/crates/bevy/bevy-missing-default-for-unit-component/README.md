# bevy-missing-default-for-unit-component

## What it does

Checks for unit structs, such as `struct Player;`, that implement `Component` but not `Default`.

## Why is this bad?

Bevy builds required components with `Default`. Without it, `#[require(Player)]` on another
component does not compile, and generic code such as `insert(T::default())` cannot use the marker.
A unit struct has no state, so `Default` costs nothing.

## Known problems

None known.

## Example

```rust
#[derive(Component)]
struct Player;
```

## Use instead

```rust
#[derive(Component, Default)]
struct Player;
```
