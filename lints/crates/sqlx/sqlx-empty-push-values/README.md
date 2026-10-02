# sqlx-empty-push-values

## What it does

Checks for an empty collection passed to `QueryBuilder::push_values`: an
empty array `[]`, `Vec::new()`, or `vec![]`, optionally borrowed.

## Why is this bad?

`push_values` writes the `VALUES` keyword before it reads the items. With no
items, the query ends in `VALUES` with no rows, which is invalid SQL. The error
only appears when the database runs the query.

## Known problems

The lint checks only an argument written as `[]`, `Vec::new()`, or `vec![]`,
optionally borrowed. It misses a collection that is empty only at runtime.

## Example

```rust
fn insert_users(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>) {
    query.push("INSERT INTO users (id) ");
    query.push_values([] as [i32; 0], |mut row, id| {
        row.push_bind(id);
    });
}
```

## Use instead

Handle the empty case before building the query:

```rust
fn insert_users(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, ids: Vec<i32>) {
    if ids.is_empty() {
        return;
    }
    query.push("INSERT INTO users (id) ");
    query.push_values(ids, |mut row, id| {
        row.push_bind(id);
    });
}
```
