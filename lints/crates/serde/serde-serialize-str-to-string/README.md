# serde-serialize-str-to-string

## What it does

Checks for `serializer.serialize_str(&value.to_string())`, where `to_string`
is `ToString::to_string`.

## Why is this bad?

`to_string` allocates a `String` only to pass it as a `&str`.
`Serializer::collect_str` takes the `Display` value directly, and serializers
such as `serde_json` write it without the extra allocation.

## Known problems

The lint misses a `to_string` result stored in a variable first, and other
ways to build the string, such as `format!`. Serde's default `collect_str`
still allocates, so the change only saves memory with serializers that
override it.

## Example

```rust
fn serialize<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&value.to_string())
}
```

## Use instead

```rust
fn serialize<S: serde::Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(value)
}
```
