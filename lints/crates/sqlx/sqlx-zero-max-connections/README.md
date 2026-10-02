# sqlx-zero-max-connections

## What it does

Checks for the integer literal `0` passed to `PoolOptions::max_connections`.

## Why is this bad?

A pool with a maximum of zero connections can never hand out a connection.
Every `acquire` and every query through the pool waits until the acquire
timeout and then fails.

## Known problems

The lint only checks the literal `0`. It misses a constant, a computed value,
or a value read from configuration.

## Example

```rust
fn pool_options() -> sqlx::pool::PoolOptions<sqlx::Postgres> {
    sqlx::pool::PoolOptions::new().max_connections(0)
}
```

## Use instead

```rust
fn pool_options() -> sqlx::pool::PoolOptions<sqlx::Postgres> {
    sqlx::pool::PoolOptions::new().max_connections(8)
}
```
