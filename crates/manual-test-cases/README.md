# manual-test-cases

## What it does

Checks for test functions that loop over a literal list of cases with `for` or
`Iterator::for_each`. A case list is an array, slice, repeated array, or `vec![...]` literal. It can also be a local variable, `const`, or `static` in the same crate that holds one, including after calls such as `.iter()` or `.into_iter()`.

## Why is this bad?

All cases share one test result. The first failing case stops the loop, so later
cases do not run, and the failure report does not name the case. `test-case`
turns each case into its own named test.

## Known problems

A function counts as a test when it has `#[test]` or its name starts with
`test_`. The lint skips functions that already use `#[test_case(...)]` or
`#[test_case::test_case(...)]`.

It warns on any loop over a literal list in a test, including setup loops that are not cases. For example, setup code pushes fixed bytes into a buffer. It does not flag case lists returned by a helper, taken from another crate, or iterated with `while let`. It does not flag test attributes from other crates, such as
`#[tokio::test]`, unless the name starts with `test_`. It reports only the first
loop in each test.

## Example

```rust
# fn parse(raw: &str) -> bool { raw == "ok" }
#[test]
fn parses_status() {
    for (raw, expected) in [("ok", true), ("no", false)] {
        assert_eq!(parse(raw), expected);
    }
}
```

## Use instead

```rust
# fn parse(raw: &str) -> bool { raw == "ok" }
use test_case::test_case;

#[test_case("ok", true)]
#[test_case("no", false)]
fn parses_status(raw: &str, expected: bool) {
    assert_eq!(parse(raw), expected);
}
```
