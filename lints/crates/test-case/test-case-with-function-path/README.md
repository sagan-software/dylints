# test-case-with-function-path

## What it does

Checks for a `with` validator in a `#[test_case(...)]` or
`#[test_matrix(...)]` output whose whole expression is a function path, such
as `with validate` or `with checks::validate`.

## Why is this bad?

test-case documents `with` for an inline closure and `using` for a named
validation function. Both run the same check here. Using `with` for a named
function makes the case look like it holds an inline assertion and hides that
the validator is shared.

## Known problems

The lint checks only a path made of identifiers and `::`. A generic path such
as `validate::<u8>`, a call, or a parenthesized expression does not trigger it.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

## Example

```rust
use test_case::test_case;

fn validate(actual: u8) {
    assert_eq!(actual, 4);
}

#[test_case(2 => with validate; "doubles")]
fn double(value: u8) -> u8 {
    value * 2
}
```

## Use instead

```rust
use test_case::test_case;

fn validate(actual: u8) {
    assert_eq!(actual, 4);
}

#[test_case(2 => using validate; "doubles")]
fn double(value: u8) -> u8 {
    value * 2
}
```
