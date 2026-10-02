# schemars-redundant-serde-skip

## What it does

Checks for a `#[schemars(skip)]` attribute next to an identical
`#[serde(skip)]` attribute on the same field or variant.

## Why is this bad?

Schemars already reads `#[serde(skip)]` when it derives `JsonSchema`. The
Schemars copy changes nothing in the schema. If someone later removes the Serde
attribute and keeps the Schemars copy, the schema omits a field that the
serializer writes.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It
misses a duplicate written with different spacing or quoting, an attribute
that holds more than one key, and a type declared inside a function body.

The lint matches any attribute key that starts with `skip`. Duplicated
`skip_serializing`, `skip_deserializing`, and `skip_serializing_if` attributes
also trigger this lint.

## Example

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Record {
    #[serde(skip)]
    #[schemars(skip)]
    internal_id: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Record {
    #[serde(skip)]
    internal_id: String,
}
```
