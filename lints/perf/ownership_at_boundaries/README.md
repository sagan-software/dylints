# ownership_at_boundaries

## What it does

Checks `pub` functions and methods for parameters of type `String` or `Vec<T>`
taken by value. Type aliases of these types also trigger the lint.

## Why is this bad?

A by-value `String` or `Vec<T>` forces a caller that only has a borrow to
allocate a copy, even when the function only reads the value. `&str` and
`&[T]` accept both owned and borrowed data without a copy.

## Known problems

The lint does not check how the parameter is used. It warns even when the
function stores, mutates, or returns the value, such as a constructor
`pub fn new(name: String) -> Self`. In those cases, taking ownership is
correct.

Functions with `pub(crate)` or narrower visibility are not checked. A `pub`
function inside a private module is checked.

## Example

```rust
pub fn count_errors(lines: Vec<String>) -> usize {
    lines.iter().filter(|line| line.contains("ERROR")).count()
}
```

## Use instead

```rust
pub fn count_errors(lines: &[String]) -> usize {
    lines.iter().filter(|line| line.contains("ERROR")).count()
}
```
