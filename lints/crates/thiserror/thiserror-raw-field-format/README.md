# thiserror-raw-field-format

## What it does

Checks for a raw identifier placeholder, such as `{r#type}`, in the format
string of an `#[error(...)]` attribute on a type that derives `thiserror::Error`.

## Why is this bad?

Thiserror 2 rejects raw identifiers in format strings. Code written for
thiserror 1 stops compiling after the upgrade. The unraw placeholder `{type}`
refers to the same `r#type` field.

## Known problems

With thiserror 2, an active item with this pattern already fails to compile
with thiserror's own error, so the lint is most useful on thiserror 1 code
before a migration.

thiserror 1 cannot format a keyword field such as `r#type` as `{type}`, so
that rewrite is offered but not applied automatically. A rewrite of a
non-keyword name such as `{r#kind}` to `{kind}` works with both versions and is
applied automatically. A format string with escape sequences gets help text
but no rewrite.

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
