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

A different default function or a bare `default` versus `default = "path"`
remains an override and does not trigger the lint.

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
