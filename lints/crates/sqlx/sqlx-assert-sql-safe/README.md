# sqlx-assert-sql-safe

## What it does

Checks for any construction of `sqlx::AssertSqlSafe`, whatever the wrapped
SQL text is.

## Why is this bad?

`AssertSqlSafe` tells SQLx to accept a dynamic SQL string as safe. SQLx does
not check or escape that string. If any part of it comes from user input, the
input can change the statement and cause SQL injection.

## Known problems

The lint flags every construction, including SQL that code builds only from
trusted values or that a reviewer has checked by hand. It misses constructions
inside a macro expansion.

## Example

```rust
fn count_rows(table: &str) -> sqlx::AssertSqlSafe<String> {
    sqlx::AssertSqlSafe(format!("SELECT count(*) FROM {table}"))
}
```

## Use instead

Keep the SQL text static and bind values as parameters. If a table or column
name must vary, pick it from a fixed list:

```rust
fn count_rows_query(table: &str) -> Option<&'static str> {
    match table {
        "users" => Some("SELECT count(*) FROM users"),
        "teams" => Some("SELECT count(*) FROM teams"),
        _ => None,
    }
}
```
