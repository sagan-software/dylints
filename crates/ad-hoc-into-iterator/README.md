# ad-hoc-into-iterator

## What it does

Checks inherent methods named `into_*`, `iter_*`, or `items` that take only a
`self`, `&self`, or `&mut self` receiver and return a type that implements
`Iterator`, such as `std::vec::IntoIter<T>`, `std::slice::Iter<'_, T>`, or
`impl Iterator`. The lint skips the method when the receiver type already
implements `IntoIterator`. The lint skips names that contain a policy word such
as `filter`, `sorted`, `unique`, `page`, `owned`, or `active`. The lint treats
methods that return a collection such as `Vec<T>` as conversions rather than
iteration views, so it skips them.

## Why is this bad?

A custom iteration method does not work with `for` loops over the value, with
`Iterator::zip` or `Extend::extend`, or with generic `T: IntoIterator` bounds.
Callers must learn the local method name instead.

## Known problems

The lint checks only the name and signature, so it warns on an iteration view
that is one of several valid views of the type. The policy words match anywhere
in the name, so the lint skips `into_pages` and `iter_validated`. For a method
that borrows, such as `iter_rows(&self)`, the help asks for
`IntoIterator for &Rows` instead of `IntoIterator for Rows`.

## Example

```rust
struct Row;

struct Rows {
    rows: Vec<Row>,
}

impl Rows {
    fn into_rows(self) -> std::vec::IntoIter<Row> {
        self.rows.into_iter()
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
