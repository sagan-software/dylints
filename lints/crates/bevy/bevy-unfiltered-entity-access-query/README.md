# bevy-unfiltered-entity-access-query

## What it does

Checks parameters of type `Query<EntityRef>` or `Query<EntityMut>` when entities yielded or fetched
directly from that query reach a typed component call: `contains`, `get`, `get_components`,
`get_components_mut`, `get_mut`, or `get_ref`. It recognizes query iteration in `for` loops and
direct `iter().for_each` or `iter_mut().for_each` closures. It also recognizes `get`, `get_mut`,
`single`, `single_mut`, `iter().next()`, and `iter_mut().next()` results after direct `unwrap`,
`expect`, `if let`, `match`, or `let-else` extraction.

Each query parameter is checked independently. An `EntityRef` or `EntityMut` obtained from another
query or from `World` does not trigger a diagnostic for this parameter.

## Why is this bad?

`EntityRef` claims read access to every component, and `EntityMut` claims write access to every
component. The system then conflicts with all systems that write any component, even though it only
uses a fixed set.

## Known problems

The lint follows only the direct origins listed above. Aliases of the `Query` parameter, computed
query receivers such as helper-call results, iterator adapters, and named callbacks passed to
`for_each` remain unknown. Tuple query data, such as `(Entity, EntityRef)`, is not checked. For an
assignment, the lint checks the right-hand side with the old origin. It then drops that origin if it
cannot prove that the assigned value comes from the selected query.

At control-flow joins, the lint does not restore lost origins. An assignment in one branch can make
later uses unknown.

## Example

```rust
fn log_positions(query: Query<EntityRef>) {
    for entity in &query {
        if let Some(position) = entity.get::<Position>() {
            info!("{}", position.0);
        }
    }
}
```

## Use instead

```rust
fn log_positions(query: Query<&Position>) {
    for position in &query {
        info!("{}", position.0);
    }
}
```
