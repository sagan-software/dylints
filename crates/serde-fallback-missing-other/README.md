# serde-fallback-missing-other

## What it does

Checks for an internally or adjacently tagged enum that derives
`Deserialize` and whose last variant is a unit variant named `Other` or
`Unknown` without `#[serde(other)]`.

## Why is this bad?

Without `#[serde(other)]`, the variant only matches the literal tag `"Other"`
or `"Unknown"`. Input with any new tag fails to deserialize, which defeats the
fallback variant's intended purpose.

## Known problems

A variant named `Other` or `Unknown` intended to match only its own tag also
triggers the lint. The lint misses fallback variants with other names or
in a position other than last.

## Example

```rust
#[derive(serde::Deserialize)]
#[serde(tag = "kind")]
enum Event {
    Created,
    Deleted,
    Unknown,
}
```

## Use instead

```rust
#[derive(serde::Deserialize)]
#[serde(tag = "kind")]
enum Event {
    Created,
    Deleted,
    #[serde(other)]
    Unknown,
}
```
