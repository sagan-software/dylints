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

The lint compares source text after swapping `schemars` for `serde`. It misses a
duplicate written with different spacing or quoting. It also misses an attribute
that holds more than one key and a type declared inside a function body.

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
