# ad_hoc_into_iterator

## What it does

Checks methods named `into_*`, `iter_*`, or `items` that take one argument,
usually the receiver, and return `Vec<T>`, `std::vec::IntoIter<T>`, or
`impl Iterator`. Names that contain a policy word such as `filter`, `sorted`,
`unique`, `page`, `owned`, or `active` are skipped. The `into_iter` method of an
`IntoIterator` implementation is skipped.

## Why is this bad?

A custom iteration method does not work with `for` loops over the value, with
`Iterator::zip` or `Extend::extend`, or with generic `T: IntoIterator` bounds.
Callers must learn the local method name instead.

## Known problems

The lint checks only the name and signature. It warns on conversions that
return a `Vec` but are not iteration, such as `fn into_bytes(self) -> Vec<u8>`
or `fn into_inner(self) -> Vec<T>`. The policy words match anywhere in the
name, so `into_pages` and `iter_validated` are skipped. Methods that return
other iterator types, such as `std::slice::Iter`, are not checked. The lint
also warns on methods in trait implementations whose names the trait fixes.

For a method that borrows, such as `iter_rows(&self)`, implement
`IntoIterator for &Rows` instead of `IntoIterator for Rows`.

## Example

```rust
struct Row;

struct Rows {
    rows: Vec<Row>,
}

impl Rows {
    fn into_rows(self) -> Vec<Row> {
        self.rows
    }
}
```

## Use instead

```rust
struct Row;

struct Rows {
    rows: Vec<Row>,
}

impl IntoIterator for Rows {
    type Item = Row;
    type IntoIter = std::vec::IntoIter<Row>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}
```
