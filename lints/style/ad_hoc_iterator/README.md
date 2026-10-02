# ad_hoc_iterator

## What it does

Checks inherent methods named `next` or `next_*` that take only a `&mut self`
receiver and return `Option<T>`, where `T` is not the `impl` type itself.
The lint skips types that already implement `Iterator`. It also skips names that
contain a word such as `state`, `status`, `transition`, `page`, `retry`, `event`,
or `advance`.

## Why is this bad?

A custom `next` method does not work with `for` loops or iterator adapters
such as `map`, `filter`, and `collect`. Callers must write the loop by hand.

## Known problems

The lint checks only the name and signature. It warns on a `next_*` method that
is one of several ways to step through the type rather than its one item
sequence. The lint skips names containing those words, so it skips
`next_statement`.

## Example

```rust
struct Row;

struct Rows {
    rows: Vec<Row>,
}

impl Rows {
    fn next_row(&mut self) -> Option<Row> {
        self.rows.pop()
    }
}
```

## Use instead

```rust
struct Row;

struct Rows {
    rows: Vec<Row>,
}

impl Iterator for Rows {
    type Item = Row;

    fn next(&mut self) -> Option<Self::Item> {
        self.rows.pop()
    }
}
```
