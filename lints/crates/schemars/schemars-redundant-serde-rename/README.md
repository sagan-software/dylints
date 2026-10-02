# schemars-redundant-serde-rename

## What it does

Checks for a `#[schemars(rename = "...")]` attribute next to the same
`#[serde(rename = "...")]` attribute on the same item, field, or variant.

## Why is this bad?

Schemars already reads `#[serde(rename)]` when it derives `JsonSchema`. The
duplicate attribute from Schemars changes nothing in the schema. When someone
later renames one copy and not the other, the schema documents a name the
serializer does not use.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It misses a
duplicate written with different spacing or quoting. It also misses an attribute
that holds more than one key and a type declared inside a function body.

A different name in the Schemars attribute is an intended override and does
not trigger the lint. A duplicated `rename_all` attribute triggers
`schemars-redundant-serde-rename-all` instead.

## Example

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Response {
    #[serde(rename = "requestId")]
    #[schemars(rename = "requestId")]
    request_id: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Response {
    #[serde(rename = "requestId")]
    request_id: String,
}
```
