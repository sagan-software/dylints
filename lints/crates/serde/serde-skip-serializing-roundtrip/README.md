# serde-skip-serializing-roundtrip

## What it does

Checks for a `#[serde(skip_serializing)]` field in a type that derives both
`Serialize` and `Deserialize` when the field has no `skip`,
`skip_deserializing`, or `default`, and the container has no `default`.

## Why is this bad?

`skip_serializing` does not skip deserializing. The serialized output leaves
the field out, but deserialization still requires it, so reading back your own
output fails with a missing field error.

## Known problems

Serde fills a missing `Option` field with `None`, so an `Option` field
round-trips without a default but still triggers the lint. A field that other
input always supplies also triggers the lint.

The lint matches derives to types by name, so two types with the same name in
one crate can share the same derive result.

## Example

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct Resource {
    name: String,
    #[serde(skip_serializing)]
    hash: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct Resource {
    name: String,
    #[serde(skip_serializing, default)]
    hash: String,
}
```
