# thiserror-redundant-backtrace-attr

## What it does

Checks for `#[backtrace]` on a field that thiserror already treats as the
backtrace. The field must be first in a `thiserror::Error` struct or variant,
have the written type name `Backtrace`, and not be a source field.

## Why is this bad?

Thiserror already uses a field with the written type name `Backtrace` as the
error's backtrace. The attribute changes nothing, and readers might expect it to
change behavior. On a source field, `#[backtrace]` forwards the source's
backtrace, so the lint skips fields named `source` and fields marked
`#[source]` or `#[from]`.

## Known problems

thiserror itself reads the written type name, so a type alias for
`Backtrace` is not a backtrace field and does not trigger the lint.

## Example

```rust
# #![feature(error_generic_member_access)]
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    #[backtrace]
    backtrace: std::backtrace::Backtrace,
}
```

## Use instead

```rust
# #![feature(error_generic_member_access)]
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    backtrace: std::backtrace::Backtrace,
}
```
