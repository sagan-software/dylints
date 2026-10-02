# test-case-singleton-matrix-dimension

## What it does

Checks for a `#[test_matrix(...)]` attribute that generates more than one test
and has an input set written as an array or tuple with exactly one value, such
as `["fixed"]` in `#[test_matrix(["fixed"], [0, 1, 2])]`.

## Why is this bad?

A one-value collection makes a constant input look like a varying one. Readers
cannot tell which inputs create the combinations, or whether values were
removed by mistake. test-case accepts a plain expression for a constant input
and generates the same tests. A matrix that generates only one test is covered
by `test-case-single-case-matrix` instead.

## Known problems

The lint checks only array and tuple literals written in the attribute. A
one-value range such as `0..1`, a constant, or a macro does not trigger it.

The fix replaces the collection with its value. When that value is itself an
array, tuple, or range, unwrapping it would create a new input set, so the lint
gives help text but no automatic fix.

## Example

```rust
use test_case::test_matrix;

#[test_matrix(["fixed"], [0, 1, 2])]
fn parses(input: &str, mode: u8) {
    assert!(!input.is_empty() && mode < 3);
}
```

## Use instead

```rust
use test_case::test_matrix;

#[test_matrix("fixed", [0, 1, 2])]
fn parses(input: &str, mode: u8) {
    assert!(!input.is_empty() && mode < 3);
}
```
