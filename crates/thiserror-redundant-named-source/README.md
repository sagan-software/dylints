# thiserror-redundant-named-source

## What it does

Checks for `#[source]` on a field named `source` in a `thiserror::Error`
struct or variant when no other field has `#[source]` or `#[from]`.

## Why is this bad?

Thiserror treats a field named `source` as the error source without an
attribute, so `#[source]` changes nothing. The extra attribute suggests that
the field would not be the source without it.

## Known problems

A field written as the raw identifier `r#source` is ordinary data for
thiserror, so `#[source]` on it is not redundant and the lint does not warn.

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
