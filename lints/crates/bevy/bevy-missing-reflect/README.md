# bevy-missing-reflect

## What it does

Checks for local types that implement `Component`, `Resource`, `Message`, or `Event` but not
`Reflect`.

## Why is this bad?

Bevy's reflection-based tools cannot see a type without `Reflect`. Scenes do not save it, the
remote protocol cannot read it, and inspectors do not show it.

## Known problems

The lint reports every such type, including internal types that no tool needs to inspect. Deriving
`Reflect` also requires every field type to implement `Reflect`, or the field must be marked
`#[reflect(ignore)]`.

## Example

```rust
#[derive(Component)]
struct Health(u32);
```

## Use instead

```rust
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Health(u32);
```
