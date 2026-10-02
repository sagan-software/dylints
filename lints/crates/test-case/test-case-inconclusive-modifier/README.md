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

test-case still supports `inconclusive`, so a project may keep it on purpose.

The lint matches text before the first `;` in the attribute. An input string
that contains `inconclusive` also triggers it.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

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
