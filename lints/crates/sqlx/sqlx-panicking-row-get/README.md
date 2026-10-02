# sqlx-panicking-row-get

## What it does

Checks for calls to `sqlx::Row::get` and `sqlx::Row::get_unchecked`.

## Why is this bad?

Both methods panic when the column does not exist or its value cannot be
decoded into the requested type. A renamed column or an unexpected `NULL` then
crashes the program instead of returning an error the caller can handle.

## Known problems

The lint flags calls where the column is known to exist and to have the right
type. It misses calls inside a macro expansion.

## Example

```rust
use sqlx::Row;

fn user_name(row: &sqlx::postgres::PgRow) -> String {
    row.get("name")
}
```

## Use instead

```rust
use sqlx::Row;

fn user_name(row: &sqlx::postgres::PgRow) -> Result<String, sqlx::Error> {
    row.try_get("name")
}
```
