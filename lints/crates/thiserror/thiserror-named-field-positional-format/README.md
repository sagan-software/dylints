# thiserror-named-field-positional-format

## What it does

Checks for an `#[error(...)]` attribute whose format string ends with an empty
`{}` placeholder and is followed by a format argument, such as
`#[error("failed: {}", source)]`.

## Why is this bad?

A positional placeholder hides which field supplies the displayed value. When
format arguments are added or reordered, a placeholder can silently show the
wrong field. Thiserror can capture a named field inside the format string, so
the extra argument is redundant.

## Known problems

The lint matches the attribute source text. It does not check that the
argument is a named field, so it also warns on arguments such as
`self.count + 1`, on messages with several `{}` placeholders, and on any
attribute named `error`, even outside a `thiserror` derive. In these cases it
gives help text but no automatic fix.

It does not warn when the `{}` placeholder is not the last part of the format
string, when the placeholder has a format specification such as `{:?}`, or when
whitespace separates the closing quote from the comma.

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
