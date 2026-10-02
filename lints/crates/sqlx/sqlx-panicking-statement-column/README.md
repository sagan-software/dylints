# sqlx-panicking-statement-column

## What it does

Checks for calls to `sqlx::Statement::column` and `sqlx::Row::column`.

## Why is this bad?

`column` panics when the index or name does not match a column of the prepared
statement or the row. A changed query then crashes the program instead of returning an
error the caller can handle.

## Known problems

The lint flags calls with an index that is known to be valid.

## Example

```rust
use sqlx::Statement;

fn first_column(statement: &sqlx::postgres::PgStatement<'_>) -> &sqlx::postgres::PgColumn {
    statement.column(0)
}
```

## Use instead

```rust
use sqlx::Statement;

fn first_column(
    statement: &sqlx::postgres::PgStatement<'_>,
) -> Result<&sqlx::postgres::PgColumn, sqlx::Error> {
    statement.try_column(0)
}
```
