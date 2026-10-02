# test-case-large-suite

## What it does

Checks for a function whose `#[test_case(...)]` and `#[test_matrix(...)]`
attributes generate 64 or more tests in total. A matrix generates one test for
each combination of its input sets.

## Why is this bad?

Each generated test adds compile time, run time, and lines in the test output.
A large Cartesian product often repeats the same behavior with many values, and
the cases that matter become hard to find.

## Known problems

The limit of 64 is fixed. A suite where every combination covers different
behavior still triggers the lint. The lint counts tests, not the time each
test takes.

## Example

```rust
use test_case::test_matrix;

#[test_matrix(0..8, 0..8)]
fn adds_commutatively(left: u8, right: u8) {
    assert_eq!(left + right, right + left);
}
```

## Use instead

Keep representative boundary values. For broad generated input, use a
property-testing crate.

```rust
use test_case::test_matrix;

#[test_matrix([0, 1, 7], [0, 1, 7])]
fn adds_commutatively(left: u8, right: u8) {
    assert_eq!(left + right, right + left);
}
```
