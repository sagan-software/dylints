# test-case-whitespace-ignore-reason

## What it does

Checks for an `ignore["..."]` or `inconclusive["..."]` modifier in a
`#[test_case(...)]` or `#[test_matrix(...)]` output whose reason is not empty
but contains only whitespace.

## Why is this bad?

The test harness skips the case. A blank reason looks like a reason in the
source but says nothing about why the case is skipped or what must change
before it can run again. It is also easy to miss in review.

## Known problems

A whitespace-only `ignore` reason also triggers `test-case-empty-ignore-reason`, so the
attribute gets two warnings. A reason from a constant or macro does not
trigger this lint.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
