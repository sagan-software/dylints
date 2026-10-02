# test-case-async-without-test-harness

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` function whose
generated test functions are still `async`, because no async runtime test
attribute such as `#[tokio::test]` follows the test-case attributes.

## Why is this bad?

test-case does not add `#[test]` to async functions. It copies the attributes
written after it, such as `#[tokio::test]`, onto every generated case. Without
such an attribute, the generated cases are plain async functions. `cargo test`
does not run them and reports no failure.

## Known problems

A custom test framework that finds async functions without transforming them
can trigger a false positive.

## Example

```rust
use test_case::test_case;

async fn double(value: u8) -> u8 {
    value * 2
}

#[test_case(1; "one")]
async fn doubles(value: u8) {
    assert_eq!(double(value).await, value * 2);
}
```

## Use instead

Place the runtime attribute after the test-case attributes.

```rust
use test_case::test_case;

async fn double(value: u8) -> u8 {
    value * 2
}

#[test_case(1; "one")]
#[tokio::test]
async fn doubles(value: u8) {
    assert_eq!(double(value).await, value * 2);
}
```
