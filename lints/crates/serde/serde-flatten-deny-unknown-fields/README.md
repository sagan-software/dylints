# serde-flatten-deny-unknown-fields

## What it does

Checks for a `#[serde(flatten)]` field in a struct that has
`#[serde(deny_unknown_fields)]`, or whose type is a struct in the crate that
has `#[serde(deny_unknown_fields)]`.

## Why is this bad?

Serde documents that `flatten` does not work with `deny_unknown_fields` on
either the outer or the flattened struct. With this combination,
deserialization can reject valid input because one struct treats the fields of
the other as unknown.

## Known problems

The lint finds the flattened struct by the last name in the field type. It
misses a flattened struct wrapped in another type, such as `Option<Extra>`, or
defined in another crate. A different struct with the same name and
`deny_unknown_fields` can cause a false positive.

The lint matches derives to types by name, so two types with the same name in
one crate can share the same derive result.

## Example

```rust
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct User {
    id: String,
    #[serde(flatten)]
    extra: Extra,
}

#[derive(serde::Deserialize)]
struct Extra {
    trace_id: String,
}
```

## Use instead

```rust
#[derive(serde::Deserialize)]
struct User {
    id: String,
    #[serde(flatten)]
    extra: Extra,
}

#[derive(serde::Deserialize)]
struct Extra {
    trace_id: String,
}
```
