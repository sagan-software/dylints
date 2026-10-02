# thiserror-redundant-from-source

## What it does

Checks for a field of a `thiserror::Error` type that has both `#[from]` and
`#[source]`.

## Why is this bad?

Thiserror treats a `#[from]` field as the error source, so `#[source]` changes
nothing. The extra attribute suggests the two attributes have different
effects on this field.

## Known problems

The lint recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

## Example

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io")]
    Io(#[from] #[source] std::io::Error),
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("io")]
    Io(#[from] std::io::Error),
}
```
