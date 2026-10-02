# test-case-ignore-without-reason

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the bare `ignore` modifier instead of `ignore["reason"]`.

## Why is this bad?

The test harness skips the case without recording a reason. Readers cannot tell
why the harness skips it or what must change before it can run again, so the
skip tends to become permanent.

## Known problems

The lint checks only the `ignore` spelling. A bare `inconclusive` modifier
triggers `test-case-inconclusive-modifier` instead.

## Example

```rust
use test_case::test_case;

#[test_case(1 => ignore; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => ignore["blocked by issue #123"]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```
