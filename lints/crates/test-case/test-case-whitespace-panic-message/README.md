# test-case-whitespace-panic-message

## What it does

Checks for a `panics "..."` output in a `#[test_case(...)]` or
`#[test_matrix(...)]` attribute whose expected message is not empty but
contains only whitespace.

## Why is this bad?

test-case checks that the panic message contains the expected text. Most panic
messages contain a space, so an unrelated `unwrap`, index, or overflow panic
can pass the case even when the intended failure never happens.

## Known problems

The attribute also triggers `test-case-empty-panic-message`, so it gets two
warnings. A message from a constant or macro does not trigger this lint.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

## Example

```rust
use test_case::test_case;

#[test_case(0 => panics " "; "zero")]
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
