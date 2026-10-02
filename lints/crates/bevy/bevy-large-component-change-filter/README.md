# bevy-large-component-change-filter

## What it does

Checks for query parameters with a `Changed<T>` filter when `T` is a large local component and the
function uses fewer than half of `T`'s named fields.

A large component has at least eight named fields and a size over 64 bytes, as in
`bevy-large-component`.

## Why is this bad?

Bevy tracks changes per component. A write to any field of `T` marks all of `T` as changed, so the
filter matches entities where none of the fields this system reads have changed.

## Known problems

Fields are matched by name anywhere in the function body, so a field with the same name on another
type counts as a use. Field accesses inside closures and in destructuring patterns are not counted.

## Example

```rust
// `Agent` has nine named fields, including `health`, and is over 64 bytes.
fn react_to_health(query: Query<&Agent, Changed<Agent>>) {
    for agent in &query {
        info!("health: {}", agent.health);
    }
}
```

## Use instead

Move the field into its own component and filter on that component.

```rust
fn react_to_health(query: Query<&Health, Changed<Health>>) {
    for health in &query {
        info!("health: {}", health.0);
    }
}
```
