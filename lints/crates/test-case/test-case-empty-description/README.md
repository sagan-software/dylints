# test-case-empty-description

## What it does

Checks for a `#[test_case(...)]` attribute whose description after the `;` is
an empty string literal, such as `#[test_case(1; "")]`.

## Why is this bad?

test-case builds the generated test name from the description. An empty
description produces a name that does not say which input or behavior the case
covers, so a failure report does not identify the case.

## Known problems

A description produced by another macro does not trigger the lint.

## Example

```rust
use test_case::test_case;

#[test_case(1; "")]
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
