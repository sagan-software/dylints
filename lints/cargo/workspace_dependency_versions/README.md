# workspace_dependency_versions

## What it does

Checks the nearest `Cargo.toml` above the crate root file of a workspace
package for dependencies that set their own `version`, `path` or `git` source instead of using
`workspace = true`. It checks `[dependencies]`, `[dev-dependencies]`,
`[build-dependencies]`, and their `[target.'...']` variants. The warning points
at the source value. When a dependency declares both version and path,
the version diagnostic takes precedence.

The lint applies only when the workspace root declares
`[workspace.dependencies]`. The workspace root is the package manifest itself
when it has a `[workspace]` table. Otherwise, an explicit `package.workspace`
path, resolved relative to the package manifest directory, selects the root.
When that key is absent, the root is the nearest ancestor `Cargo.toml` with a
`[workspace]` table that does not exclude the package.

## Why is this bad?

When each package sets its own versions, one workspace can depend on several
versions of the same crate. Upgrades and audits then need an edit in every
manifest instead of one entry in `[workspace.dependencies]`.

## Known problems

The lint reports explicit sources even when the workspace has no entry for
that dependency. Move the source into `[workspace.dependencies]` first.

Cargo validates malformed dependency values. This lint reports string
`version`, `path` and `git` values.

A package that needs two versions of one crate must give each version its own
renamed key in `[workspace.dependencies]`, such as
`bevy_018 = { package = "bevy", version = "0.18.0" }`.

## Example

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[workspace.dependencies]
anyhow = "1.0.0"

[dependencies]
anyhow.workspace = true
serde = "1.0.0"
```

## Use instead

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[workspace.dependencies]
anyhow = "1.0.0"
serde = "1.0.0"

[dependencies]
anyhow.workspace = true
serde.workspace = true
```
