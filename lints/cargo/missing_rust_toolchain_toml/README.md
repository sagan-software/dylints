# missing_rust_toolchain_toml

## What it does

Checks for a `rust-toolchain.toml` file in the package directory or a parent
directory up to the workspace root. The workspace root is the package manifest
itself when it has a `[workspace]` table. Otherwise, it is the nearest ancestor
`Cargo.toml` with a `[workspace]` table that does not exclude the package. The
warning points at the `[package]` header of the manifest.

## Why is this bad?

Without a pinned toolchain, each contributor and CI job builds with whatever
Rust version the machine provides. Compiler, Clippy, rustfmt, and Dylint results
can then differ between machines.

## Known problems

Only the exact name `rust-toolchain.toml` counts. A bare `rust-toolchain`
file does not satisfy the lint; `rust_toolchain_toml` reports that file. The
lint ignores a `rust-toolchain.toml` above the workspace root. It does not read
a `package.workspace` key that points at the workspace root.

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
├── rust-toolchain.toml
└── src/lib.rs
```
