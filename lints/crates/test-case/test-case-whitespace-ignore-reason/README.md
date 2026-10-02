# test-case-whitespace-ignore-reason

## What it does

Checks for an `ignore["..."]` or `inconclusive["..."]` modifier in a
`#[test_case(...)]` or `#[test_matrix(...)]` output whose reason is not empty
but contains only whitespace.

## Why is this bad?

The test harness skips the case. A blank reason looks like a reason in the
source but gives no explanation of the skip or condition for rerunning it.
Reviewers can easily miss the blank content.

## Known problems

A reason from a constant or macro does not trigger the lint. An empty reason
triggers `test-case-empty-ignore-reason` instead.

## Example

```rust
use test_case::test_case;

#[test_case(1 => ignore["  "]; "one")]
fn validates(value: u8) {
    assert_eq!(value, 1);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => ignore["blocked by issue #123"]; "one")]
fn validates(value: u8) {
    assert_eq!(value, 1);
}
```
