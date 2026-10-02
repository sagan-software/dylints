# serde-borrow-redundant-str-bytes

## What it does

Checks for `#[serde(borrow)]` on a `&str` or `&[u8]` field of a type that
derives `Deserialize`, including through a type alias defined in the crate.

## Why is this bad?

Serde always borrows `&str` and `&[u8]` fields from the input. The attribute
changes nothing on these types, and readers can mistake it for a needed
setting.

## Known problems

The lint misses a type alias from another crate.

The machine-applicable fix deletes the whole attribute, so it is offered only
when `borrow` is the attribute's only entry. An attribute such as
`#[serde(borrow, rename = "name")]` gets help without a fix.

The lint matches derives to types by name, so two types with the same name in
one crate can share the same derive result.

## Example

```rust
#[derive(serde::Deserialize)]
struct User<'a> {
    #[serde(borrow)]
    name: &'a str,
}
```

## Use instead

```rust
#[derive(serde::Deserialize)]
struct User<'a> {
    name: &'a str,
}
```
