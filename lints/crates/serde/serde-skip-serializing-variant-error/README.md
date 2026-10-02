# serde-skip-serializing-variant-error

## What it does

Checks for an enum variant with `#[serde(skip)]` or
`#[serde(skip_serializing)]` in an enum that derives `Serialize`.

## Why is this bad?

Serde returns an error when code serializes a variant marked `skip` or
`skip_serializing`. The enum compiles as serializable, but serializing that
one variant fails at runtime.

## Known problems

Code that never serializes the variant still triggers the lint.

The lint matches derives to types by name, so two types with the same name in
one crate can share the same derive result.

## Example

```rust
#[derive(serde::Serialize)]
enum Event {
    Sent,
    #[serde(skip_serializing)]
    Internal,
}
```

## Use instead

Remove the attribute so the variant serializes:

```rust
#[derive(serde::Serialize)]
enum Event {
    Sent,
    Internal,
}
```
