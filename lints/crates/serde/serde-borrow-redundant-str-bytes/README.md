# serde-borrow-redundant-str-bytes

## What it does

Checks for `#[serde(borrow)]` on a field of a type that derives
`Deserialize` when the field type is `&str`, `&[u8]`, or an `Option` of either.

## Why is this bad?

Serde always borrows fields written as `&str` and `&[u8]`, or `Option` of
either, from the input. The attribute
changes nothing on these types, and readers can mistake it for a needed
setting.

## Known problems

Serde decides implicit borrowing from the written type, so a type alias for
`&str` still needs `#[serde(borrow)]` and the lint does not flag it.

The machine-applicable fix deletes the whole attribute, so the lint offers it
only when `borrow` is the attribute's only entry. An attribute such as
`#[serde(borrow, rename = "name")]` gets help without a fix.

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
