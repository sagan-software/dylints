# thiserror-no-std-path-display

## What it does

Checks for an `#[error(...)]` format string that captures a `Path` or `PathBuf`
field as `{field}` when the package's `Cargo.toml` sets
`default-features = false` on `thiserror` without enabling its `std` feature.

## Why is this bad?

Thiserror displays `Path` and `PathBuf` fields only when its `std` feature is
enabled. Without it, the derive fails with a missing `Display` implementation
that does not mention the feature.

## Known problems

The lint scans source and manifest text instead of resolved types and
features. It reads only inline dependency entries such as
`thiserror = { version = "2", default-features = false }` in the nearest
`Cargo.toml`, so it misses a `[dependencies.thiserror]` table. It warns even
when another dependency enables thiserror's `std` feature through Cargo feature
unification.

It treats a field as a path when any line in the file declares that field name
with a type containing `Path`, so a same-named field in another struct can
cause a warning. It checks only the exact `{field}` capture, not `{field:<10}`
or a positional argument.

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
