# thiserror-redundant-backtrace-attr

## What it does

Checks for `#[backtrace]` on the field that thiserror already uses as the
backtrace: the first field of a `thiserror::Error` struct or variant whose
written type is named `Backtrace`, when that field is not a source field.

## Why is this bad?

Thiserror already provides a field whose type is named `Backtrace` as the
error's backtrace. The attribute changes nothing, and readers may expect it to
change behavior. On a source field, `#[backtrace]` forwards the source's
backtrace, so the lint skips fields named `source` and fields marked
`#[source]` or `#[from]`.

## Known problems

thiserror itself reads the written type name, so a type alias for
`Backtrace` is not a backtrace field and does not trigger the lint.

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
