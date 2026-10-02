# thiserror-source-field-opt-out

## What it does

Checks for a field named `source` whose type is `String`, `&str`, `str`,
`char`, `bool`, or a primitive integer, in a file that derives an `Error` type.

## Why is this bad?

Thiserror treats a field named `source` as the error's `source()`. When the
field holds ordinary data, the derive fails because the type does not
implement `std::error::Error`. Thiserror 2 accepts the raw identifier
`r#source` as an ordinary data field.

## Known problems

The lint scans source text line by line. It warns on any line that starts with
`source:` or `pub source:` followed by one of the listed types, anywhere in a
file that contains a derive ending in `Error`. A matching field in another
struct, or a function parameter on its own line, can trigger it. It also warns
on items disabled by `#[cfg]`.

It does not warn for other data types such as `Option<String>` or `f64`, or for
visibility such as `pub(crate) source`.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("{source} -> {destination}")]
pub struct RouteError {
    source: char,
    destination: char,
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error("{source} -> {destination}")]
pub struct RouteError {
    r#source: char,
    destination: char,
}
```
