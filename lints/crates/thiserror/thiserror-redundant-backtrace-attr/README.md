# thiserror-redundant-backtrace-attr

## What it does

Checks for `#[backtrace]` on a field of a `thiserror::Error` type when the
field's type is named `Backtrace` and the field is not a source field.

## Why is this bad?

Thiserror already provides a field whose type is named `Backtrace` as the
error's backtrace. The attribute changes nothing, and readers may expect it to
change behavior. On a source field, `#[backtrace]` forwards the source's
backtrace, so the lint skips fields named `source` and fields marked
`#[source]` or `#[from]`.

## Known problems

The lint checks only the last segment of the written type path, so any type
named `Backtrace` matches and a type alias does not.

It recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    #[backtrace]
    backtrace: std::backtrace::Backtrace,
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    backtrace: std::backtrace::Backtrace,
}
```
