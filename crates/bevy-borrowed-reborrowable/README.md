# bevy-borrowed-reborrowable

## What it does

Checks for function parameters that take a reborrowable Bevy type through `&mut`, such as
`&mut Query<..>`, `&mut Commands`, or `&mut ResMut<..>`.

The checked types are `Commands`, `Deferred`, `DeferredWorld`, `EntityCommands`, `EntityMut`,
`FilteredEntityMut`, `Mut`, `MutUntyped`, `NonSendMut`, `PtrMut`, `Query`, and `ResMut`.

## Why is this bad?

These types already hold a mutable borrow and provide `reborrow`. Wrapping them in another `&mut`
adds a second layer of indirection, and callers must keep a mutable binding alive just to pass it.

## Known problems

The lint skips `self` parameters and parameters whose lifetime appears in the return type. It also
skips trait method implementations because the trait fixes their parameter types.

## Example

```rust
# use bevy::ecs::{component::Component, system::Query};
# #[derive(Component)]
# struct Marker;
fn count_markers(query: &mut Query<&Marker>) -> usize {
    query.iter().count()
}
```

## Use instead

```rust
# use bevy::ecs::{component::Component, system::Query};
# #[derive(Component)]
# struct Marker;
fn count_markers(query: Query<&Marker>) -> usize {
    query.iter().count()
}

fn system(mut query: Query<&Marker>) {
    let _count = count_markers(query.reborrow());
}
```
