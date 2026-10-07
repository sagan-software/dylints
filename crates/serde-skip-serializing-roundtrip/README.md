# serde-skip-serializing-roundtrip

## What it does

Checks for a `#[serde(skip_serializing)]` field in a type that derives both
`Serialize` and `Deserialize` when the field has no `skip`,
`skip_deserializing`, or `default`, and the container has no `default`. An
`Option` field is not checked unless it has `with` or `deserialize_with`,
because Serde fills a missing `Option` field with `None`.

## Why is this bad?

`skip_serializing` does not skip deserializing. The serialized output leaves
the field out, but deserialization still requires it, so reading back your own
output fails with a missing field error.

## Known problems

A field that other input always supplies also triggers the lint.

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
