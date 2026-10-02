# test-case-empty-panic-message

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
`panics ""`, an empty expected panic message.

## Why is this bad?

test-case checks that the panic message contains the expected text. Every
message contains the empty string, so the case passes on any panic. An
unrelated `unwrap`, index, or overflow panic can pass the case even when the
intended failure never happens.

## Known problems

A message from a constant or macro does not trigger the lint. A
whitespace-only message triggers `test-case-whitespace-panic-message` instead.

## Example

```rust
use test_case::test_case;

#[test_case(0 => panics ""; "zero")]
fn reciprocal(value: u8) {
    let _ = 1 / value;
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(0 => panics "attempt to divide by zero"; "zero")]
fn reciprocal(value: u8) {
    let _ = 1 / value;
}
```
