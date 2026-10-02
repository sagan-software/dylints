# thiserror-no-std-path-display

## What it does

Checks for an `#[error(...)]` format string that captures a `Path` or
`PathBuf` field as `{field}` when the compiled package's `Cargo.toml`
disables thiserror's default features without enabling its `std` feature.

## Why is this bad?

Thiserror's `Path` and `PathBuf` display support requires its `std` feature.
Without it, the derive fails with a missing `Display` implementation that does
not mention the feature.

## Known problems

The lint reads the manifest for the package Cargo compiles, including
`[dependencies.thiserror]` tables, `[target.*]` tables, renamed
dependencies, and `workspace = true` declarations. It still warns when
another dependency enables thiserror's `std` feature through Cargo feature
unification.

The lint runs before type checking, because the missing feature makes type
checking fail. It therefore recognizes a field type by its written name, so a
type alias for `PathBuf` does not trigger it. It checks only the exact
`{field}` capture; a capture with a format spec needs `Display` in every
configuration.

## Example

```rust
#[derive(thiserror::Error, Debug)]
#[error("missing {path}")]
pub struct Error {
    path: std::path::PathBuf,
}
```

## Use instead

Enable thiserror's `std` feature, or format the path with `display()`.

```rust
#[derive(thiserror::Error, Debug)]
#[error("missing {}", path.display())]
pub struct Error {
    path: std::path::PathBuf,
}
```
