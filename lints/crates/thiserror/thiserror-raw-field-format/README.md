# thiserror-raw-field-format

## What it does

Checks for a raw identifier placeholder, such as `{r#type}`, in the format
string of an `#[error(...)]` attribute on a type that derives `thiserror::Error`.

## Why is this bad?

Thiserror 2 rejects raw identifiers in format strings. Code written for
thiserror 1 stops compiling after the upgrade. The unraw placeholder `{type}`
refers to the same `r#type` field.

## Known problems

With thiserror 2, an active item with this pattern already fails to compile,
so the lint is most useful during a migration from thiserror 1. It also scans
source text, so it warns on items disabled by `#[cfg]`.

It recognizes the derive only as `thiserror::Error` or through a
`use thiserror::Error` or `use thiserror::Error as Name` import. A glob import
such as `use thiserror::*` hides the derive from the lint.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("bad {r#type}")]
pub struct Error {
    r#type: String,
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error("bad {type}")]
pub struct Error {
    r#type: String,
}
```
