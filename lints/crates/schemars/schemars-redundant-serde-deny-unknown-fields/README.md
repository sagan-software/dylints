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
