# test-case-ignore-without-reason

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the bare `ignore` modifier instead of `ignore["reason"]`.

## Why is this bad?

The test harness skips the case and records no reason. Readers cannot tell why
the case is skipped or what must change before it can run again, so the skip
tends to become permanent.

## Known problems

The lint matches text before the first `;` in the attribute. It warns when
that text contains `ignore` but not `ignore[`. An input such as
`"ignore me"` triggers it, and so does `ignore ["reason"]` written with a
space before the bracket.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
