# thiserror-source-field-opt-out

## What it does

Checks for a field named `source` in a type that derives `thiserror::Error`
when no field has `#[source]` or `#[from]` and the field's written type is
`String`, `str`, `&str`, `char`, `bool`, or a primitive number.

## Why is this bad?

Thiserror treats a field named `source` as the error's `source()`. When the
field holds ordinary data, the derive fails because the type does not
implement `std::error::Error`. Thiserror 2 accepts the raw identifier
`r#source` as an ordinary data field.

## Known problems

The lint runs before type checking, because the field makes type checking
fail. It therefore reads the written type name: a local type named `String`
that implements `Error` triggers it, and a type alias for a primitive does
not. It does not warn for other data types such as `Option<String>`.

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
