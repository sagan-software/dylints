# workspace-lints-inheritance

## What it does

Checks the nearest `Cargo.toml` above the crate root file of a workspace
package. If the manifest defines `lints` without `workspace = true` and the
workspace root declares `[workspace.lints]`, the lint warns at the package's
`lints` key or table header. A `[lints]` table, a `[lints.*]` table, a dotted
key, and an inline table all count.

The workspace root is the package manifest itself when it has a `[workspace]`
table. Otherwise, an explicit `package.workspace` path, resolved relative to
the package manifest directory, selects the root. When that key is absent, the
root is the nearest ancestor `Cargo.toml` with a `[workspace]` table that does
not exclude the package.

If an explicit path cannot select a workspace root, the lint does not search
ancestor workspaces.

## Why is this bad?

A package with its own lint table does not inherit `[workspace.lints]`.
Changes to the shared lint policy then miss that package without any warning.

## Known problems

Manifests with no `lints` key do not warn; `package_lints_section` reports
them.

## Example

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[workspace.lints.rust]
unsafe_code = "forbid"

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
