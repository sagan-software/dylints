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

The lint only recognizes `std::borrow::Cow`, `alloc::borrow::Cow`, a name
imported from one of them with `use`, and a type alias defined in the crate.
It misses `Cow` imported through a glob and `Cow` nested in another type,
such as `Option<Cow<'a, str>>`.

A field that should always own its data also triggers the lint.

The lint matches derives to types by name, so two types with the same name in
one crate can share the same derive result.

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
