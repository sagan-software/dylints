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

The lint compares parsed Serde and Schemars entries on the same AST node. Keys
must match exactly, and string values compare after Rust decodes literal
escapes. The lint visits loaded modules, local items, variants, and fields.
Unsupported or malformed keys and value forms do not trigger the lint.

For `rename_all`, a direct rule applies to both serialization and
deserialization. Direction-specific rules compare by direction.

A fix removes the whole Schemars attribute when its only entry is redundant and
comment-free. For one redundant entry in a mixed attribute, the fix deletes
only that entry and one adjacent comma, including a legal trailing comma.
Comments outside the entry remain. The lint warns without a fix when the
matching entry contains a comment, multiple redundant entries share one
attribute, or the matching Schemars attribute comes from macro expansion.

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
