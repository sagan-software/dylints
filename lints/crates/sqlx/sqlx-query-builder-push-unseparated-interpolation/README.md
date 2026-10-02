# sqlx-query-builder-push-unseparated-interpolation

## What it does

Checks for a `format!(...)` call, optionally borrowed, passed directly to
`Separated::push_unseparated`. Qualified forms such as `std::format!` also
match.

## Why is this bad?

`push_unseparated` adds text to the SQL statement as is. Formatting a value
into that text skips bind parameters. If the value comes from user input, it
can change the statement and cause SQL injection.

## Known problems

The lint checks only an argument that is the direct result of `format!`,
optionally borrowed. It misses a formatted string stored in a variable first.
It also flags `format!` calls that only insert fixed SQL fragments.

## Example

```rust
fn filter_by_ids(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, first: i64, second: i64) {
    query.push("WHERE id IN (");
    let mut ids = query.separated(", ");
    ids.push_bind(first);
    ids.push_unseparated(format!(", {second})"));
}
```

## Use instead

```rust
fn filter_by_ids(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, first: i64, second: i64) {
    query.push("WHERE id IN (");
    let mut ids = query.separated(", ");
    ids.push_bind(first);
    ids.push_bind(second);
    ids.push_unseparated(")");
}
```
