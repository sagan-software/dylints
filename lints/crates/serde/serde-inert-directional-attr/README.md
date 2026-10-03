# serde-inert-directional-attr

## What it does

Checks for a Serde attribute that only affects one direction on a type that
derives only the other direction. On a type that derives only `Serialize`, it
checks `alias`, `default`, `deserialize_with`, `borrow`, and
`skip_deserializing`. On a type that derives only `Deserialize`, it checks
`skip_serializing`, `skip_serializing_if`, `serialize_with`, and
`getter`.

## Why is this bad?

The derive ignores the attribute, so it has no effect. Readers expect it to
change behavior, and the author might have intended to derive the other
direction.

## Known problems

The lint reports inactive `serialize` and `deserialize` string values in
`rename`, `rename_all`, `rename_all_fields`, and `bound`. If a plain
`#[serde(...)]` source attribute has a unique supported direction entry and an
exact deletion span without comments, the lint offers a machine edit that
preserves active directional values. Otherwise, it gives help text without an
edit.

## Example

```rust
#[derive(serde::Serialize)]
struct Output {
    #[serde(alias = "old_value")]
    value: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize)]
struct Output {
    value: String,
}
```
