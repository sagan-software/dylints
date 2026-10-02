# workspace_lints_inheritance

## What it does

Checks the nearest `Cargo.toml` above the crate root file of a workspace
package. If the manifest has a `[lints]` or `[lints.*]` table without
`workspace = true` under `[lints]`, the lint warns.

## Why is this bad?

A package with its own lint table does not inherit `[workspace.lints]`.
Changes to the shared lint policy then miss that package without any warning.

## Known problems

The warning points at the crate root source file.

A package counts as a workspace package when its manifest has a `[workspace]`
table, uses a key such as `version.workspace = true`, or has an ancestor
`Cargo.toml` with a `[workspace]` table. A single-package `[workspace]` used
only to stop Cargo from searching parent directories therefore also triggers
the lint.

The lint does not check that `[workspace.lints]` exists. Manifests with no
`[lints]` table do not warn; `package_lints_section` reports them.

## Example

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[lints.rust]
unsafe_code = "forbid"
```

## Use instead

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[workspace.lints.rust]
unsafe_code = "forbid"

[lints]
workspace = true
```
