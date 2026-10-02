# bevy-missing-copy-for-unit-component

## What it does

Checks for unit structs, such as `struct Player;`, that implement `Component` but not `Copy`.

## Why is this bad?

Without `Copy`, passing the marker by value moves it, and generic code with a `Copy` bound cannot
use it. A unit struct has no state, so `Copy` costs nothing.

## Known problems

None known.

## Example

```rust
#[derive(Component, Clone)]
struct Player;
```

## Use instead

```rust
#[derive(Component, Clone, Copy)]
struct Player;
```
