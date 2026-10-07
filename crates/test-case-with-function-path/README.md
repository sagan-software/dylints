# test-case-with-function-path

## What it does

Checks for a `with` validator in a `#[test_case(...)]` or
`#[test_matrix(...)]` output whose whole expression is a function path, such
as `with validate` or `with checks::validate`.

## Why is this bad?

test-case documents `with` for an inline closure and `using` for a named
validation function. Both run the same check here. Using `with` for a named
validation function makes the case look like it holds an inline assertion and
hides the validator's reuse across cases.

## Known problems

The lint checks only a path, including a generic path such as
`validate::<u8>`. A call or a parenthesized expression does not trigger it.
The fix renames `with` to `using`, which calls the same function.

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
