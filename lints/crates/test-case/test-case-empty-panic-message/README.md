# test-case-empty-panic-message

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
`panics ""`, an expected panic message that is empty or whitespace-only.

## Why is this bad?

test-case checks that the panic message contains the expected text. Every
message contains the empty string, so the case passes on any panic. An
unrelated `unwrap`, index, or overflow panic can pass the case even when the
intended failure never happens.

## Known problems

The lint matches the attribute source text after removing whitespace. A
whitespace-only message such as `panics " "` also triggers
`test-case-whitespace-panic-message`, so the attribute gets two warnings. A raw
string such as `panics r""` does not trigger this lint.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
