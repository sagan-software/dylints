# serde-flatten-deny-unknown-fields

## What it does

Checks for a `#[serde(flatten)]` field in a struct that derives
`Deserialize` when the struct has `#[serde(deny_unknown_fields)]`, or when
the field type, or the `T` of an `Option<T>` field, is a struct in the crate
that has `#[serde(deny_unknown_fields)]`.

## Why is this bad?

Serde documents that `flatten` does not work with `deny_unknown_fields` on
either the outer or the flattened struct. With this combination,
deserialization can reject valid input because one struct treats the fields of
the other as unknown.

## Known problems

The lint reads `deny_unknown_fields` only from structs defined in the crate,
so it misses a flattened struct from another crate.

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
