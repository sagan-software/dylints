# test-case-constant-match-guard

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` output of the form
`matches PATTERN if GUARD` where the whole guard is the literal `true` or
`false`.

## Why is this bad?

An `if true` guard adds no condition, so the case checks less than it appears
to. An `if false` guard rejects every value, so the case always fails.

## Known problems

The lint checks only a guard that is the literal `true` or `false`. A named
constant, `!false`, `(true)`, or a compound expression does not trigger it.

For `if true`, the fix removes the guard. An `if false` guard gets help text
but no automatic fix, because the intended condition is unknown.

## Example

```rust
use test_case::test_case;

#[test_case(1 => matches Some(_) if true; "some")]
fn wrap(value: u8) -> Option<u8> {
    Some(value)
}
```

## Use instead

Remove a `true` guard. Replace a `false` guard with the intended condition.

```rust
use test_case::test_case;

#[test_case(1 => matches Some(value) if value > 0; "some")]
fn wrap(value: u8) -> Option<u8> {
    Some(value)
}
```
