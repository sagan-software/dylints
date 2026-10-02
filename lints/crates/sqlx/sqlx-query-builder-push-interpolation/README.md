# sqlx-query-builder-push-interpolation

## What it does

Checks for a `format!(...)` call, optionally borrowed, passed directly to
`QueryBuilder::push`. Qualified forms such as `std::format!` also match.

## Why is this bad?

`push` adds text to the SQL statement as is. Formatting a value into that text
skips bind parameters. If the value comes from user input, it can change the
statement and cause SQL injection.

## Known problems

The lint checks only an argument that is the direct result of `format!`,
optionally borrowed. It misses a formatted string stored in a variable first.
It also flags `format!` calls that only insert fixed SQL fragments.

## Example

```rust
fn filter_by_id(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, id: i64) {
    query.push(format!("WHERE id = {id}"));
}
```

## Use instead

```rust
fn filter_by_id(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, id: i64) {
    query.push("WHERE id = ").push_bind(id);
}
```
