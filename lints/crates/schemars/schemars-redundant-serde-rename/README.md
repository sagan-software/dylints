# schemars-redundant-serde-rename

## What it does

Checks for a `#[schemars(rename = "...")]` attribute next to an identical
`#[serde(rename = "...")]` attribute on the same item, field, or variant.

## Why is this bad?

Schemars already reads `#[serde(rename)]` when it derives `JsonSchema`. The
Schemars copy changes nothing in the schema. When someone later renames one
copy and not the other, the schema documents a name the serializer does not
use.

## Known problems

The lint compares source text after swapping `schemars` for `serde`. It
misses a duplicate written with different spacing or quoting, an attribute
that holds more than one key, and a type declared inside a function body.

The lint matches any attribute key that starts with `rename`. A duplicated
`rename_all` attribute triggers this lint as well as
`schemars-redundant-serde-rename-all`. A different name in the Schemars
attribute is an intended override and does not trigger the lint.

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
