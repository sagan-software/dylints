# ad_hoc_iterator

## What it does

Checks inherent methods named `next` or `next_*` that take only a `&mut self`
receiver and return `Option<T>`, where `T` is not the `impl` type itself.
Types that already implement `Iterator` are skipped. Names that contain a word
such as `state`, `status`, `transition`, `page`, `retry`, `event`, or `advance`
are skipped.

## Why is this bad?

A custom `next` method does not work with `for` loops or iterator adapters
such as `map`, `filter`, and `collect`. Callers must write the loop by hand.

## Known problems

The lint checks only the name and signature. It warns on a `next_*` method that
is one of several ways to step through the type rather than its one item
sequence. The skipped words match anywhere in the name, so `next_statement` is
skipped.

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
