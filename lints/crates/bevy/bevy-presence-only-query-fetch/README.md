# bevy-presence-only-query-fetch

## What it does

Checks for parameters of type `Query<&T>` that the function uses only through `is_empty()` or
`iter().count()`.

## Why is this bad?

The system never reads `T`, but `&T` still adds read access to `T`. The system then conflicts with
systems that write `T`. A `With<T>` filter matches the same entities without that access.

## Known problems

Any other use of the query prevents the warning. The lint follows query uses into closure bodies, so
a closure that reads a fetched component also prevents the warning. The lint recognizes only a
single shared component reference, `&T`, as query data.

## Example

```rust
# use bevy_ecs::prelude::{Component, Query};
# #[derive(Component)]
# struct Enemy(u32);
# macro_rules! info { ($($args:tt)*) => { println!($($args)*); }; }
fn count_enemies(query: Query<&Enemy>) {
    let count = query.iter().count();
    info!("{count} enemies");
}
```

## Use instead

```rust
# use bevy_ecs::prelude::{Component, Entity, Query, With};
# #[derive(Component)]
# struct Enemy(u32);
# macro_rules! info { ($($args:tt)*) => { println!($($args)*); }; }
fn count_enemies(query: Query<Entity, With<Enemy>>) {
    let count = query.iter().count();
    info!("{count} enemies");
}
```
