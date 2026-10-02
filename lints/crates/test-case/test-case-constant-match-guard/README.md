# test-case-constant-match-guard

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` output of the form
`matches PATTERN if GUARD` where the whole guard is the literal `true` or
`false`.

## Why is this bad?

An `if true` guard adds no condition, so the case checks less than it appears
to. An `if false` guard rejects every value, so the case always fails.

## Known problems

The lint checks only a guard that is a single `true` or `false` token. A named
constant, `!false`, `(true)`, or a compound expression does not trigger it.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
