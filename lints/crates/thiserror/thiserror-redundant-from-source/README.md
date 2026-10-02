# thiserror-redundant-from-source

## What it does

Checks for a field of a `thiserror::Error` type that has both `#[from]` and
`#[source]`.

## Why is this bad?

Thiserror treats a `#[from]` field as the error source, so `#[source]` changes
nothing. The extra attribute suggests that `#[source]` changes how thiserror
treats the field, although `#[from]` already marks it as the source.

## Known problems

None known.

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
