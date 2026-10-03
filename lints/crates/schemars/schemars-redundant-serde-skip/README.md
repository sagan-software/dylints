# schemars-redundant-serde-skip

## What it does

Checks for a `#[schemars(skip)]` attribute next to the same
`#[serde(skip)]` attribute on the same field or variant. The lint also checks
the `skip_serializing`, `skip_deserializing`, and `skip_serializing_if` keys,
which Schemars also reads from Serde.

## Why is this bad?

Schemars already reads `#[serde(skip)]` and the other skip keys when it
derives `JsonSchema`. The duplicate attribute from Schemars changes nothing in
the schema. If someone later removes the Serde attribute and keeps the Schemars
copy, the schema omits a field that the serializer writes.

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
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Record {
    #[serde(skip)]
    #[schemars(skip)]
    internal_id: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, schemars::JsonSchema)]
struct Record {
    #[serde(skip)]
    internal_id: String,
}
```
