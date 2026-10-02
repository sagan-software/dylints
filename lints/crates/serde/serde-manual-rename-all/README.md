# serde-manual-rename-all

## What it does

Checks for a struct with at least two named fields where every field has
`#[serde(rename = "...")]` and all names follow one `rename_all` convention.
It checks each derived direction and also handles
`rename(serialize = "...", deserialize = "...")`.

## Why is this bad?

A container-level `rename_all` states the convention once. Repeating the
rename on every field adds noise, and a new field added without a rename breaks
the convention.

## Known problems

The lint only detects `PascalCase`, `camelCase`,
`SCREAMING_SNAKE_CASE`, `kebab-case`, and `SCREAMING-KEBAB-CASE`. It skips a
struct where any field has no rename, and it does not check enum variants or
tuple structs.

The machine-applicable fix is offered only when each field attribute holds
just the `rename` entry.

## Example

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct User {
    #[serde(rename = "firstName")]
    first_name: String,
    #[serde(rename = "lastName")]
    last_name: String,
}
```

## Use instead

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    first_name: String,
    last_name: String,
}
```
