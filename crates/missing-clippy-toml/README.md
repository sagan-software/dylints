# missing-clippy-toml

## What it does

Checks for a `clippy.toml` or `.clippy.toml` file in Clippy's configuration
search path. Clippy chooses `CLIPPY_CONF_DIR` when set, then
`CARGO_MANIFEST_DIR` when set, then the current directory. It searches the
chosen directory and each parent through the filesystem root. Clippy checks
`.clippy.toml` before `clippy.toml` in each directory. It does not fall back to
a lower-priority start directory if the chosen directory has no configuration
file.

This lint checks only whether the file exists. Clippy validates its contents.

The warning points at the package manifest's `[package]` header.

## Why is this bad?

Without a checked-in `clippy.toml`, Clippy uses its defaults or a
configuration file outside the repository. Different machines and CI can then
report different Clippy results for the same code.

## Known problems

The lint skips crates without a package manifest, such as files compiled
directly with `rustc`.

## Example

```text
my-crate/
├── Cargo.toml
└── src/lib.rs
```

## Use instead

```text
my-crate/
├── Cargo.toml
├── clippy.toml
└── src/lib.rs
```
