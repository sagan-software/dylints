# insta-deprecated-assert-display

## What it does

Checks for `insta::assert_display_snapshot!` calls.

## Why is this bad?

Insta deprecated `assert_display_snapshot!`. `assert_snapshot!` takes the same
`Display` values and writes the same snapshots. A deprecated macro can be
removed in a future major release, which breaks the test build.

## Known problems

None known.

## Example

```rust
fn snapshot_body(body: &str) {
    insta::assert_display_snapshot!(body);
}
```

## Use instead

```rust
fn snapshot_body(body: &str) {
    insta::assert_snapshot!(body);
}
```
