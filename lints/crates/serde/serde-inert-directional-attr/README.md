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

The lint checks only the keys listed above. It does not report a directional
form such as `rename(deserialize = "name")` on a `Serialize`-only type.

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
