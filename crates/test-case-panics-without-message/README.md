# test-case-panics-without-message

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the bare `panics` modifier without an expected message.

## Why is this bad?

The case passes on any panic. An unrelated `unwrap`, index, or overflow panic
can pass the case even when the intended failure never happens.

## Known problems

Some tests only need to check that a function panics, because the message is
not stable. The lint warns on those tests too.

test-case 3.3 drops a bare `panics` that a description follows, as in
`1 => panics ; "name"`, so the generated test no longer expects a panic. The
lint reports this form too.

## Example

```rust
use test_case::test_case;

#[test_case(0 => panics; "zero")]
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
