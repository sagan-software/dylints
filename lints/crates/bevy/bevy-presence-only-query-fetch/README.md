# bevy-presence-only-query-fetch

## What it does

Checks for parameters of type `Query<&T>` that the function uses only through `is_empty()` or
`iter().count()`.

## Why is this bad?

The system never reads `T`, but `&T` still adds read access to `T`. The system then conflicts with
systems that write `T`. A `With<T>` filter matches the same entities without that access.

## Known problems

Any other use of the query prevents the warning. The lint does not inspect uses inside closures, so it can report a query even if a closure also reads it. The lint does not check query data other than a single `&T`.

## Example

```rust
fn count_enemies(query: Query<&Enemy>) {
    let count = query.iter().count();
    info!("{count} enemies");
}
```

## Use instead

```rust
fn count_enemies(query: Query<Entity, With<Enemy>>) {
    let count = query.iter().count();
    info!("{count} enemies");
}
```
