# test-case-inconclusive-modifier

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the `inconclusive` modifier.

## Why is this bad?

test-case generates an ignored test for an `inconclusive` case. The test
harness reports it as ignored, not as a separate inconclusive result.
`ignore["reason"]` has the same effect and uses the name the test output
shows.

## Known problems

test-case still supports `inconclusive`, so a project might keep it on purpose.
The fix renames the keyword to `ignore` and keeps any reason.

## Example

```rust
use test_case::test_case;

#[test_case(1 => inconclusive["issue 123"]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => ignore["issue 123"]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```
