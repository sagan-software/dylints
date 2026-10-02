# test-case-nonpositive-almost-precision

## What it does

Checks for an `is almost` or `is almost_equal_to` matcher in a
`#[test_case(...)]` or `#[test_matrix(...)]` output whose `precision` is a zero
or negative numeric literal.

## Why is this bad?

test-case checks `(actual - expected).abs() < precision`. An absolute
difference is never less than zero, so the case fails for every finite value,
even when the two values are equal.

## Known problems

The lint checks only a numeric literal without a type suffix, with or without
a leading `-`. A literal such as `0.0_f64`, a constant, or an arithmetic
expression does not trigger it.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

## Example

```rust
use test_case::test_case;

#[test_case(1.0 => is almost 1.0 precision 0.0; "same value")]
fn identity(value: f64) -> f64 {
    value
}
```

## Use instead

Use a positive tolerance. If the values must be exactly equal, use the plain
`=> expected` form.

```rust
use test_case::test_case;

#[test_case(1.0 => is almost 1.0 precision 0.001; "same value")]
fn identity(value: f64) -> f64 {
    value
}
```
