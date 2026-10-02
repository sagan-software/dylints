# sqlx-panicking-row-get

## What it does

Checks for calls to `sqlx::Row::get` and `sqlx::Row::get_unchecked`.

## Why is this bad?

Both methods panic when the column does not exist or when SQLx cannot decode its
value into the target type. A renamed column or an unexpected `NULL` then
crashes the program instead of returning an error the caller can handle.

## Known problems

The lint flags calls even when the column exists and its value has the expected
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
