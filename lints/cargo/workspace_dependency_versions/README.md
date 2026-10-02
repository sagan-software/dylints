# workspace_dependency_versions

## What it does

Checks the nearest `Cargo.toml` above the crate root file of a workspace
package for dependencies that set their own `version` instead of using
`workspace = true`. It checks `[dependencies]`, `[dev-dependencies]`,
`[build-dependencies]`, and their `[target.'...']` variants.

## Why is this bad?

When each package sets its own versions, one workspace can depend on several
versions of the same crate. Upgrades and audits then need an edit in every
manifest instead of one entry in `[workspace.dependencies]`.

## Known problems

The warning points at the crate root source file. The message names the
dependency.

A package counts as a workspace package when its manifest has a `[workspace]`
table, uses a key such as `version.workspace = true`, or has an ancestor
`Cargo.toml` with a `[workspace]` table. A single-package `[workspace]` used
only to stop Cargo from searching parent directories therefore also triggers
the lint.

Dev-dependencies are not checked when the manifest declares an `[[example]]`
target.

Dependencies with only `path` or `git` and no `version` do not warn. The
manifest is read line by line, so multiline inline tables and quoted
dependency names can be skipped or misread.

## Example

```toml
[package]
name = "example"
version = "0.1.0"
edition = "2024"

[workspace]

[dependencies]
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
serde = "1.0.0"

[dependencies]
serde.workspace = true
```
