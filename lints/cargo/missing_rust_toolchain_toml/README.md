# missing_rust_toolchain_toml

## What it does

Checks that a `rust-toolchain.toml` file exists in the crate directory or in a
parent directory up to the workspace root. The workspace root is the nearest
ancestor whose `Cargo.toml` has a `[workspace]` table.

## Why is this bad?

Without a pinned toolchain, each contributor and CI job builds with whatever
Rust version is installed. Compiler, Clippy, rustfmt, and Dylint results can
then differ between machines.

## Known problems

The warning points at the crate root source file.

Only the exact name `rust-toolchain.toml` counts. A bare `rust-toolchain`
file does not satisfy the lint; `rust_toolchain_toml` reports that file. A
`rust-toolchain.toml` above the workspace root is ignored.

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
