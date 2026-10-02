# schemars-redundant-serde-rename-all

## What it does

Checks for a `#[schemars(rename_all = "...")]` attribute next to the same
`#[serde(rename_all = "...")]` attribute on the same struct or enum.

## Why is this bad?

Schemars already reads `#[serde(rename_all)]` when it derives `JsonSchema`.
The duplicate attribute from Schemars changes nothing in the schema. When
someone later changes one copy and not the other, the schema documents names the
serializer does not use.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It misses a
duplicate written with different spacing or quoting. It also misses an attribute
that holds more than one key and a type declared inside a function body.

A different case convention in the Schemars attribute is an intended override
and does not trigger the lint.

## Example

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(rename_all = "camelCase")]
struct Response {
    request_id: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
struct Response {
    request_id: String,
}
```
