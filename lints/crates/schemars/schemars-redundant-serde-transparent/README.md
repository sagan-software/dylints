# schemars-redundant-serde-transparent

## What it does

Checks for a `#[schemars(transparent)]` attribute next to an identical
`#[serde(transparent)]` attribute on the same struct.

## Why is this bad?

Schemars already reads `#[serde(transparent)]` when it derives `JsonSchema`.
The Schemars copy changes nothing in the schema. If someone later removes the
Serde attribute and keeps the Schemars copy, the schema can describe a
different shape than the serializer writes.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It
misses a duplicate written with different spacing or quoting, an attribute
that holds more than one key, and a type declared inside a function body.

## Example

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
struct Id(String);
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(transparent)]
struct Id(String);
```
