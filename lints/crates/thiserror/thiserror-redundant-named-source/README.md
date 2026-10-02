# thiserror-redundant-named-source

## What it does

Checks for `#[source]` on a field named `source` in a `thiserror::Error` type.

## Why is this bad?

Thiserror treats a field named `source` as the error source without an
attribute, so `#[source]` changes nothing. The extra attribute suggests that
the field would not be the source without it.

## Known problems

A field written as the raw identifier `r#source` does not trigger the lint.

The lint recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    #[source]
    source: std::io::Error,
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error("failed")]
pub struct Error {
    source: std::io::Error,
}
```
