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

The lint compares parsed Serde and Schemars entries on the same AST node. Keys
must match exactly, and string values compare after Rust decodes literal
escapes. The lint visits loaded modules, local items, variants, and fields.
Unsupported or malformed keys and value forms do not trigger the lint.

For `rename`, a direct name applies to both serialization and deserialization.
Direction-specific names compare by direction.

A fix removes the whole Schemars attribute when its only entry is redundant and
comment-free. For one redundant entry in a mixed attribute, the fix deletes
only that entry and one adjacent comma, including a legal trailing comma.
Comments outside the entry remain. The lint warns without a fix when the
matching entry contains a comment, multiple redundant entries share one
attribute, or the matching Schemars attribute comes from macro expansion.

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
