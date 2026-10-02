# bevy-zst-query

## What it does

Checks for function parameters of type `Query<..>` whose query data contains a `&T` or `&mut T`
where `T` is a zero-sized type, such as a unit marker component.

## Why is this bad?

A reference to a zero-sized component carries no data. It still adds access to `T` to the system,
so the system conflicts with systems that write `T`. A `With<T>` filter matches the same entities
without that access.

## Known problems

The lint only checks references directly in the query data or in tuples. It does not look inside
`Option<&T>` or custom `QueryData` types.

## Example

```rust
fn move_players(mut query: Query<(&mut Transform, &Player)>) {
    for (mut transform, _player) in &mut query {
        transform.translation.x += 1.0;
    }
}
```

## Use instead

```rust
fn move_players(mut query: Query<&mut Transform, With<Player>>) {
    for mut transform in &mut query {
        transform.translation.x += 1.0;
    }
}
```
