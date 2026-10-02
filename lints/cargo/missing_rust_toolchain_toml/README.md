# missing_rust_toolchain_toml

## What it does

Checks that a `rust-toolchain.toml` file exists in the package directory or in
a parent directory up to the workspace root. The workspace root is the package
manifest itself when it has a `[workspace]` table, or the nearest ancestor
`Cargo.toml` with a `[workspace]` table that does not exclude the package. The
warning points at the `[package]` header of the manifest.

## Why is this bad?

Without a pinned toolchain, each contributor and CI job builds with whatever
Rust version is installed. Compiler, Clippy, rustfmt, and Dylint results can
then differ between machines.

## Known problems

Only the exact name `rust-toolchain.toml` counts. A bare `rust-toolchain`
file does not satisfy the lint; `rust_toolchain_toml` reports that file. A
`rust-toolchain.toml` above the workspace root is ignored. A
`package.workspace` key that points at the workspace root is not read.

Crates without a package manifest, such as files compiled directly with
`rustc`, are not checked.

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
