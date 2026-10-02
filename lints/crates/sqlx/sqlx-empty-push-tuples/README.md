# sqlx-empty-push-tuples

## What it does

Checks for an empty collection passed to `QueryBuilder::push_tuples`: an
empty array `[]`, `Vec::new()`, or `vec![]`, optionally borrowed.

## Why is this bad?

`push_tuples` writes the parentheses around the tuple list before it reads the
items. With no items, the query contains an empty `()` list, which is invalid
SQL. The error only appears when the database runs the query.

## Known problems

The lint checks only an argument written as `[]`, `Vec::new()`, or `vec![]`,
optionally borrowed. It misses a collection that is empty only at runtime.

## Example

```rust
fn find_pairs(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>) {
    query.push("SELECT * FROM pairs WHERE (a, b) IN ");
    query.push_tuples([] as [(i32, i32); 0], |mut tuple, (a, b)| {
        tuple.push_bind(a).push_bind(b);
    });
}
```

## Use instead

Handle the empty case before building the query:

```rust
fn find_pairs(query: &mut sqlx::QueryBuilder<'_, sqlx::Postgres>, pairs: Vec<(i32, i32)>) {
    if pairs.is_empty() {
        return;
    }
    query.push("SELECT * FROM pairs WHERE (a, b) IN ");
    query.push_tuples(pairs, |mut tuple, (a, b)| {
        tuple.push_bind(a).push_bind(b);
    });
}
```
