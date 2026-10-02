# thiserror-transparent-single-source-display

## What it does

Checks for a `thiserror::Error` struct or variant whose only field is its
source and whose message only displays that field, such as `#[error("{0}")]`
or `#[error("{source}")]`. The field counts as the source when it has
`#[from]` or `#[source]`, or when it is named `source`.

## Why is this bad?

The wrapper displays the inner error's message and also returns the inner
error from `source()`. A reporter that prints the whole error chain then shows
the same message twice. `#[error(transparent)]` forwards both `Display` and
`source()` to the inner error.

## Known problems

The suggested change alters `source()`. After the change, the wrapper's
`source()` returns the inner error's source instead of the inner error. Code
that downcasts the wrapper's source to the inner type stops matching, so the
suggestion is never applied automatically.

thiserror rejects `#[source]` on the field of a transparent item, so a field
with `#[source]` gets help without a suggestion. Remove `#[source]` by hand
when you apply the change.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("{source}")]
pub struct Error {
    #[from]
    source: std::io::Error,
}
```

## Use instead

```rust
#[derive(thiserror::Error, Debug)]
#[error(transparent)]
pub struct Error {
    #[from]
    source: std::io::Error,
}
```
