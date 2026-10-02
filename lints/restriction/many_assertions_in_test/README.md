# many_assertions_in_test

## What it does

Checks for `#[test]` functions that contain four or more calls to the standard
`assert`, `assert_eq`, `assert_ne`, `debug_assert`, `debug_assert_eq`, or
`debug_assert_ne` macros, including through `use` renames.

## Why is this bad?

A test with many assertions often checks one structured output piece by piece.
The first failing assertion stops the test, so the report shows one difference
at a time. When the output changes on purpose, each assertion needs a manual
edit. A snapshot assertion with `insta` shows the full difference and updates
with one command.

## Known problems

It warns when the assertions check independent behavior that a snapshot would
not describe well. It counts each macro call once, so an assertion inside a loop
counts once. It does not count assertions in helper functions, local macros
named `assert_eq`, or other assertion macros such as `assert_matches`. It skips
test attributes from other crates, such as `#[tokio::test]`.

## Example

```rust
#[test]
fn renders_report() {
    let report = render_report();
    assert!(report.contains("Summary"));
    assert!(report.contains("Total"));
    assert!(report.contains("Details"));
    assert!(report.contains("Footer"));
}
```

## Use instead

```rust
#[test]
fn renders_report() {
    let report = render_report();
    insta::assert_snapshot!(report);
}
```
