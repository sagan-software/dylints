# package_lints_section

## What it does

Checks the nearest `Cargo.toml` above the crate root file. If that manifest has
a `[package]` table but no top-level `lints` key, the lint warns at the
`[package]` header. A `[lints]` or `[lints.*]` table, a dotted key such as
`lints.workspace = true`, and an inline table such as
`lints = { workspace = true }` all satisfy the lint.

## Why is this bad?

A package without a `[lints]` table does not inherit `[workspace.lints]` and
uses rustc and Clippy defaults. Lint policy then differs between crates
without any visible setting in the manifest.

## Known problems

An empty `[lints]` table satisfies the lint even though it configures nothing.

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
