# missing_clippy_toml

## What it does

Checks that a `clippy.toml` or `.clippy.toml` file exists in the package
directory or in a parent directory up to the workspace root. The workspace
root is the package manifest itself when it has a `[workspace]` table, or the
nearest ancestor `Cargo.toml` with a `[workspace]` table that does not exclude
the package. The warning points at the `[package]` header of the manifest.

## Why is this bad?

Without a checked-in `clippy.toml`, Clippy uses its defaults or a
configuration file outside the repository. Different machines and CI can then
report different Clippy results for the same code.

## Known problems

A `CLIPPY_CONF_DIR` setting, which Clippy accepts, does not satisfy the lint.
A `clippy.toml` above the workspace root is ignored. A `package.workspace`
key that points at the workspace root is not read.

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
├── clippy.toml
└── src/lib.rs
```
