# test-case-empty-matrix

## What it does

Checks for a `#[test_matrix(...)]` function that generates no test cases
because one input set, such as `[]` or the range `0..0`, is empty.

## Why is this bad?

The Cartesian product of the input sets is empty, so test-case generates no
tests and the function body never runs. `cargo test` reports no failure.

## Known problems

The lint counts every case generated for the function. When the function also
has a `#[test_case(...)]` attribute or another nonempty `#[test_matrix(...)]`
attribute, the empty matrix is not reported.

## Example

```rust
use test_case::test_matrix;

#[test_matrix([], [1, 2])]
fn parses(_input: &str, _expected: u8) {}
```

## Use instead

```rust
use test_case::test_matrix;

#[test_matrix(["zero", "one"], [1, 2])]
fn parses(_input: &str, _expected: u8) {}
```
