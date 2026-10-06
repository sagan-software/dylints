# sqlx-zero-max-connections

## What it does

Checks `PoolOptions::max_connections` calls whose limit is statically known to
be zero, including values resolved from local constants and supported unsigned
arithmetic.

## Why is this bad?

A pool with a maximum of zero connections can never hand out a connection.
Every `acquire` and every query through the pool waits until the acquire
timeout and then fails.

## Known problems

The lint resolves integer literals, local non-trait constants, and unsigned
`+`, `-`, `*`, `/`, or `%` expressions through 16 nested steps. It reports
only values it can prove are zero. Runtime configuration and other runtime
locals remain unknown. Casts, statics, trait or external constants, unsupported
operators, and arithmetic that overflows, underflows, or divides by zero also
remain unknown.

## Example

The snippets use SQLx's PostgreSQL API. UI tests use a local SQLx fixture that omits this API.

```rust,ignore
fn pool_options() -> sqlx::pool::PoolOptions<sqlx::Postgres> {
    sqlx::pool::PoolOptions::new().max_connections(0)
}
```

## Use instead

```rust,ignore
fn pool_options() -> sqlx::pool::PoolOptions<sqlx::Postgres> {
    sqlx::pool::PoolOptions::new().max_connections(8)
}
```
