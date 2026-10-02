# sqlx-unchecked-query-macro

## What it does

Checks for the SQLx macros `query_unchecked!`, `query_as_unchecked!`,
`query_scalar_unchecked!`, `query_file_unchecked!`,
`query_file_as_unchecked!`, and `query_file_scalar_unchecked!`.

## Why is this bad?

The unchecked macros still parse the SQL and count its parameters and columns
at compile time, but they skip the type checks on bind parameters and result
columns. A type mismatch then fails at runtime instead of at compile time.

## Known problems

The lint also flags unchecked macros used on purpose, for example for a
database type that SQLx cannot map to a Rust type.

## Example

```rust
async fn user_exists(pool: &sqlx::PgPool, id: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query_unchecked!("SELECT id FROM users WHERE id = $1", id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}
```

## Use instead

```rust
async fn user_exists(pool: &sqlx::PgPool, id: i64) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!("SELECT id FROM users WHERE id = $1", id)
        .fetch_optional(pool)
        .await?;
    Ok(row.is_some())
}
```
