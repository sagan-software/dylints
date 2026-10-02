# serde-cow-missing-borrow

## What it does

Checks for a `Cow<'a, str>` or `Cow<'a, [u8]>` field without
`#[serde(borrow)]` in a type that derives `Deserialize`. The lint skips fields
with a `'static` lifetime.

## Why is this bad?

Serde only borrows a `Cow` field when it has `#[serde(borrow)]`. Without it,
Serde always deserializes the field as `Cow::Owned`, so Serde allocates and
copies every value even though the type has a lifetime for borrowing.

## Known problems

Serde recognizes a borrowable `Cow` by its written name, so the lint checks
only a field whose type path ends in `Cow` and resolves to the standard
`Cow`. Serde does not borrow through a renamed import, a type alias, or a
wrapper such as `Option<Cow<'a, str>>`, so the lint skips those fields.

A field intended to own its data also triggers the lint.

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
