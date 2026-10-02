# thiserror-named-field-positional-format

## What it does

Checks for an `#[error(...)]` attribute on a type that derives
`thiserror::Error` when a `{}` placeholder takes a positional argument that is
just the name of a field, such as `#[error("failed: {}", source)]`.

## Why is this bad?

A positional placeholder hides which field supplies the displayed value. When
format arguments are added or reordered, a placeholder can silently show the
wrong field. Thiserror can capture a named field inside the format string, so
the extra argument is redundant.

## Known problems

The fix moves the field name into the placeholder and deletes the argument.
It applies only when the message has exactly one positional placeholder and
one positional argument, and the format string has no escape sequences.
Messages with several positional placeholders get help text but no fix.

## Example

```rust
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed: {}", source)]
    Operation { source: std::io::Error },
}
```

## Use instead

```rust
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("failed: {source}")]
    Operation { source: std::io::Error },
}
```
