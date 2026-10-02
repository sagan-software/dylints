# serde-untagged-missing-expecting

## What it does

Checks for a `#[serde(untagged)]` enum that derives `Deserialize` and has
no `#[serde(expecting = "...")]` message.

## Why is this bad?

When no variant matches, an untagged enum fails with a generic error such as
"data did not match any variant of untagged enum Value". The error does not
say what input is accepted. An `expecting` message replaces it with a
description of valid input.

## Known problems

The lint does not check `#[serde(untagged)]` on a single variant.

## Example

```rust
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum Value {
    Name(String),
    Id(u64),
}
```

## Use instead

```rust
#[derive(serde::Deserialize)]
#[serde(untagged, expecting = "a name string or a numeric id")]
enum Value {
    Name(String),
    Id(u64),
}
```
