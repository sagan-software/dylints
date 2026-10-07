# sqlx-pool-connection-leak

## What it does

Checks for calls to `sqlx::pool::PoolConnection::leak`.

## Why is this bad?

SQLx documents that `leak` treats the connection as checked out forever, so
each call lowers the pool's maximum size by one. After enough calls, the pool
has no connections left, and every later `acquire` waits until it times out.

## Known problems

The lint flags every call, including one that keeps a connection out of the
pool on purpose. It misses calls inside a macro expansion.

## Example

The snippets use SQLx's PostgreSQL API. UI tests use a local SQLx fixture that omits this API.

```rust,ignore
fn take_connection(
    connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
) -> sqlx::PgConnection {
    connection.leak()
}
```

## Use instead

`detach` also returns the connection but lets the pool open a replacement:

```rust,ignore
fn take_connection(
    connection: sqlx::pool::PoolConnection<sqlx::Postgres>,
) -> sqlx::PgConnection {
    connection.detach()
}
```
