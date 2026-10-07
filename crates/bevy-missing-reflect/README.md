# bevy-missing-reflect

## What it does

Checks for local types that implement `Component`, `Resource`, `Message`, or `Event` but not
`Reflect`.

## Why is this bad?

Bevy's reflection-based tools cannot see a type without `Reflect`. Scenes do not save it, the
remote protocol cannot read it, and inspectors do not show it.

## Known problems

The lint skips crates that rustc compiles as a test harness (`--test`), such as unit and
integration test builds. Test-only types do not reach scenes, inspectors or the remote protocol. The
ordinary build of the same target still checks types outside `#[cfg(test)]`.

The lint reports every such type, including internal types that no tool needs to inspect. Deriving
`Reflect` also requires every field type to implement `Reflect`, or the code must mark the field `#[reflect(ignore)]`.

## Example

```rust
# use bevy::prelude::*;
#[derive(Component)]
struct Health(u32);
```

## Use instead

```rust
# use bevy::prelude::*;
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Health(u32);
```
