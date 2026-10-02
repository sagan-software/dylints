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

The lint skips `self` parameters and parameters whose lifetime appears in the return type. It still
reports trait method implementations, where the trait fixes the parameter type.

## Example

```rust
fn count_markers(query: &mut Query<&Marker>) -> usize {
    query.iter().count()
}
```

## Use instead

```rust
fn count_markers(query: Query<&Marker>) -> usize {
    query.iter().count()
}

fn system(mut query: Query<&Marker>) {
    let _count = count_markers(query.reborrow());
}
```
