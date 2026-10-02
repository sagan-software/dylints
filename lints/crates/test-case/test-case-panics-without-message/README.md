# test-case-panics-without-message

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the bare `panics` modifier without an expected message.

## Why is this bad?

The case passes on any panic. An unrelated `unwrap`, index, or overflow panic
can pass the case even when the intended failure never happens.

## Known problems

The lint matches text before the first `;` in the attribute. It warns when no
`"` follows the first `panics`, so `panics EXPECTED_MESSAGE` with a constant
also triggers it.

Some tests only need to check that a function panics, because the message is
not stable. The lint warns on those tests too.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
