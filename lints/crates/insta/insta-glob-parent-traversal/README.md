# insta-glob-parent-traversal

## What it does

Checks for two-argument `insta::glob!` calls whose pattern is a string literal
equal to `..` or starting with `../`.

## Why is this bad?

Insta does not support parent directory traversal in the two-argument form of
`glob!`, so the glob does not find the intended files. The three-argument form
takes a base directory, which can be a parent directory.

## Known problems

The lint checks only a string literal passed directly as the pattern. It does
not check patterns with `..` after the first segment, such as
`fixtures/../other/*.txt`.

## Example

```rust
fn snapshot_fixtures() {
    insta::glob!("../fixtures/*.txt", |path| {
        insta::assert_snapshot!(std::fs::read_to_string(path).unwrap());
    });
}
```

## Use instead

```rust
fn snapshot_fixtures() {
    insta::glob!("..", "fixtures/*.txt", |path| {
        insta::assert_snapshot!(std::fs::read_to_string(path).unwrap());
    });
}
```
