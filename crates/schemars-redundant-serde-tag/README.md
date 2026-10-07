# schemars-redundant-serde-tag

## What it does

Checks for a `#[schemars(tag = "...")]` attribute next to the same
`#[serde(tag = "...")]` attribute on the same enum.

## Why is this bad?

Schemars already reads `#[serde(tag)]` when it derives `JsonSchema`. The
duplicate attribute from Schemars changes nothing in the schema. When someone
later renames one copy and not the other, the schema documents a tag field the
serializer does not write.

## Known problems

The lint compares parsed Serde and Schemars entries on the same AST node. Keys
must match exactly, and string values compare after Rust decodes literal
escapes. The lint visits loaded modules, local items, variants, and fields.
Unsupported or malformed keys and value forms do not trigger the lint.

A fix removes the whole Schemars attribute when its only entry is redundant and
comment-free. For one redundant entry in a mixed attribute, the fix deletes
only that entry and one adjacent comma, including a legal trailing comma.
Comments outside the entry remain. The lint warns without a fix when the
matching entry contains a comment, multiple redundant entries share one
attribute, or the matching Schemars attribute comes from macro expansion.

A different tag name in the Schemars attribute is an intended override and
does not trigger the lint.

## Example

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(tag = "kind")]
#[schemars(tag = "kind")]
enum Event {
    Created,
}
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
#[serde(tag = "kind")]
enum Event {
    Created,
}
```
