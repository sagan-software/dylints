# package_lints_section

## What it does

Checks the nearest `Cargo.toml` above the crate root file. If that manifest has
a `[package]` table but no `[lints]` or `[lints.*]` table, the lint warns.

## Why is this bad?

A package without a `[lints]` table does not inherit `[workspace.lints]` and
uses rustc and Clippy defaults. Lint policy then differs between crates
without any visible setting in the manifest.

## Known problems

The warning points at the crate root source file.

Only table headers count. A top-level `lints.workspace = true` or
`lints = { workspace = true }` is valid Cargo syntax but still triggers the
lint.

## Example

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"
```

## Use instead

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[lints]
workspace = true
```
