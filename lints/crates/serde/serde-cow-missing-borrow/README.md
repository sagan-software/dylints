# serde-cow-missing-borrow

## What it does

Checks for a `Cow<'a, str>` or `Cow<'a, [u8]>` field without
`#[serde(borrow)]` in a type that derives `Deserialize`. Fields with a
`'static` lifetime are not checked.

## Why is this bad?

Serde only borrows a `Cow` field when it has `#[serde(borrow)]`. Without it,
the field is always deserialized as `Cow::Owned`, so every value is copied
into a new allocation even though the type has a lifetime for borrowing.

## Known problems

Serde recognizes a borrowable `Cow` by its written name, so the lint checks
only a field whose type path ends in `Cow` and resolves to the standard
`Cow`. Serde does not borrow through a renamed import, a type alias, or a
wrapper such as `Option<Cow<'a, str>>`, so the lint skips those fields.

A field that should always own its data also triggers the lint.

## Example

```rust
use std::borrow::Cow;

#[derive(serde::Deserialize)]
struct Comment<'a> {
    body: Cow<'a, str>,
}
```

## Use instead

```rust
use std::borrow::Cow;

#[derive(serde::Deserialize)]
struct Comment<'a> {
    #[serde(borrow)]
    body: Cow<'a, str>,
}
```
