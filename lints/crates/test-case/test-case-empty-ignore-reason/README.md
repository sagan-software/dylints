# test-case-empty-ignore-reason

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` attribute that uses
the `ignore[""]` modifier with an empty or whitespace-only reason.

## Why is this bad?

The case is generated but skipped by the test harness. An empty reason does
not say why the case is skipped or what must change before it can run again,
so the case can stay disabled indefinitely.

## Known problems

The lint matches the attribute source text after removing whitespace. A
whitespace-only reason such as `ignore[" "]` also triggers
`test-case-whitespace-ignore-reason`, so the attribute gets two warnings. An
empty `inconclusive[""]` reason or a raw string such as `ignore[r""]` does not
trigger this lint.

The lint reads only the first test-case attribute on a function. The same
problem in a later attribute on that function is not reported.

## Example

```rust
use test_case::test_case;

#[test_case(1 => ignore[""]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```

## Use instead

```rust
use test_case::test_case;

#[test_case(1 => ignore["fails on Windows, see issue #123"]; "one")]
fn check(value: u8) {
    assert_eq!(value, 1);
}
```
