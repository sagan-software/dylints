# test-case-empty-ignore-reason

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
an `ignore[""]` or `inconclusive[""]` modifier with an empty reason.

## Why is this bad?

The case is generated but skipped by the test harness. An empty reason does
not say why the case is skipped or what must change before it can run again,
so the case can stay disabled indefinitely.

## Known problems

A reason from a constant or macro does not trigger the lint. A
whitespace-only reason triggers `test-case-whitespace-ignore-reason` instead.

## Example

```rust
use test_case::test_case;

#[test_case(1 => ignore[""]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => ignore["fails on Windows, see issue #123"]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```
