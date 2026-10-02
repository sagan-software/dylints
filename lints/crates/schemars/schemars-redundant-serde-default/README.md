# schemars-redundant-serde-default

## What it does

Checks for a `#[schemars(default)]` attribute next to the same
`#[serde(default)]` attribute on the same struct, enum, or field.

## Why is this bad?

Schemars already reads `#[serde(default)]` when it derives `JsonSchema`. The
duplicate attribute from Schemars changes nothing in the schema. If someone
later removes the Serde attribute and keeps the Schemars copy, the schema shows
a default that the deserializer does not apply.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It misses a
duplicate written with different spacing or quoting. It also misses an attribute
that holds more than one key and a type declared inside a function body.

## Example

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Request {
    #[serde(default)]
    #[schemars(default)]
    id: String,
}
```

## Use instead

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct Request {
    #[serde(default)]
    id: String,
}
```
