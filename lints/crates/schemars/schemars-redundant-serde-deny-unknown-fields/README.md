# schemars-redundant-serde-deny-unknown-fields

## What it does

Checks for a `#[schemars(deny_unknown_fields)]` attribute next to the same
`#[serde(deny_unknown_fields)]` attribute on the same struct or enum.

## Why is this bad?

Schemars already reads `#[serde(deny_unknown_fields)]` when it derives
`JsonSchema`. The duplicate attribute from Schemars changes nothing in the
schema. If someone later removes the Serde attribute and keeps the Schemars
copy, the schema rejects fields that the deserializer accepts.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It misses a
duplicate written with different spacing or quoting. It also misses an attribute
that holds more than one key and a type declared inside a function body.

## Example

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(deny_unknown_fields)]
struct Request {
    id: String,
}
```

## Use instead

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
}
```
