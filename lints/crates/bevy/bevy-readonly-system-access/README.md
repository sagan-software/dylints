# bevy-readonly-system-access

## What it does

Checks for query parameters with `&mut T` in their data that the function uses only through
read-only query methods: `as_readonly`, `contains`, `get`, `get_many`, `is_empty`, `iter`,
`iter_combinations`, `iter_many`, `many`, `par_iter`, and `single`.

## Why is this bad?

`&mut T` gives the system write access to `T`. Bevy then cannot run it in parallel with any other
system that reads or writes `T`, even though this system only reads.

## Known problems

Any other use of the query prevents the warning, including `&query` in a `for` loop. The lint follows
query uses into closure bodies, so a mutation inside a closure prevents the warning. It recognizes
mutable references directly in the query data or in tuples.

## Example

```rust
# use bevy::ecs::{component::Component, system::Query};
# #[derive(Component)]
# struct Position(f32);
fn log_positions(query: Query<&mut Position>) {
    for position in query.iter() {
        let _position = position.0;
    }
}
```

## Use instead

```rust
# use bevy::ecs::{component::Component, system::Query};
# #[derive(Component)]
# struct Position(f32);
fn log_positions(query: Query<&Position>) {
    for position in query.iter() {
        let _position = position.0;
    }
}
```
