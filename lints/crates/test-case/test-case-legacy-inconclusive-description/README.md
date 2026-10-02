# test-case-legacy-inconclusive-description

## What it does

Checks for a `#[test_case(...)]` or `#[test_matrix(...)]` description that
starts with the word `inconclusive` followed by a space, `-`, or `:`, such as
`"inconclusive - blocked by issue #123"`.

## Why is this bad?

Before test-case 2.0, `inconclusive` in a description marked the case as
ignored. Test-case 2.0 removed that behavior. The description is now only a
name, so the case runs even though its name says it is skipped.

## Known problems

The lint matches only lowercase `inconclusive` at the start of the
description. A description such as `"Inconclusive - ..."` does not trigger it,
although old test-case versions matched the word in any case.

## Example

```rust
use test_case::test_case;

#[test_case("letters"; "inconclusive - parser not implemented")]
fn parses(input: &str) {
    assert!(input.parse::<u8>().is_ok());
}
```

## Use instead

Skip the case with the `ignore` modifier and describe the behavior.

```rust
use test_case::test_case;

#[test_case("letters" => ignore["parser support tracked in issue #123"]; "parses letters")]
fn parses(input: &str) {
    assert!(input.parse::<u8>().is_ok());
}
```
