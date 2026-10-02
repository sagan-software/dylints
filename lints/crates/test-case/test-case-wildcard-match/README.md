# test-case-wildcard-match

## What it does

Checks for a `matches` output in a `#[test_case(...)]` or
`#[test_matrix(...)]` attribute whose whole pattern is `_` or `(_)`, with or
without an `if` guard.

## Why is this bad?

The `_` pattern matches every value, so the case passes whatever the function
returns. The case cannot detect a change in the result.

## Known problems

The lint checks only a pattern that is `_` or `(_)`. Other patterns that match
every value, such as a binding `value` or `Ok(_) | Err(_)`, do not trigger it.

## Example

```rust
use test_case::test_case;

#[test_case(1 => matches _; "one")]
fn wrap(value: u8) -> Option<u8> {
    Some(value)
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => matches Some(1); "one")]
fn wrap(value: u8) -> Option<u8> {
    Some(value)
}
```
