# test-case-unnamed-test-case

## What it does

Checks for a `#[test_case(...)]` attribute without a description after a `;`,
such as `#[test_case(1)]`.

## Why is this bad?

Without a description, test-case builds the test name from the argument
expressions. These names can be long and unclear in test output, and they
change whenever someone rewrites an input expression.

## Known problems

A function with many generated cases gets one warning for each attribute.

## Example

```rust
use test_case::test_case;

#[test_case(1)]
fn accepts_positive(value: u8) {
    assert!(value > 0);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1; "minimum positive value")]
fn accepts_positive(value: u8) {
    assert!(value > 0);
}
```
