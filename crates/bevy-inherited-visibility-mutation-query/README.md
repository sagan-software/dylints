# bevy-inherited-visibility-mutation-query

## What it does

Checks for function parameters of type `Query<..>` whose query data contains
`&mut InheritedVisibility`.

## Why is this bad?

Bevy computes `InheritedVisibility` from each entity's `Visibility` and its parent's
`InheritedVisibility`. A direct write makes it disagree with the hierarchy, and the next visibility
propagation for that entity overwrites it.

## Known problems

The lint does not look inside `Option<&mut InheritedVisibility>` or custom `QueryData` types.

## Example

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Enemy;
fn hide(mut query: Query<&mut InheritedVisibility, With<Enemy>>) {
    for mut visibility in &mut query {
        *visibility = InheritedVisibility::HIDDEN;
    }
}
```

## Use instead

```rust
# use bevy::prelude::*;
# #[derive(Component)] struct Enemy;
fn hide(mut query: Query<&mut Visibility, With<Enemy>>) {
    for mut visibility in &mut query {
        *visibility = Visibility::Hidden;
    }
}
```
