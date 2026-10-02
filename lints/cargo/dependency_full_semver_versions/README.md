# dependency_full_semver_versions

## What it does

Checks the nearest `Cargo.toml` above the crate root file for dependency
versions that omit the patch part, such as `"1"` or `"0.1"`. It checks
`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, the same
tables under `[target.'...']`, and `[workspace.dependencies]`. The warning
points at the version string and suggests the full version.

## Why is this bad?

Cargo reads `"1"` as `^1.0.0`, so it accepts every `1.x` release, including
releases older than the code needs. Reviewers cannot see which minimum version
the crate uses in tests.

## Known problems

The lint only flags plain numeric versions with one or two parts. Versions with
operators, wildcards, or pre-release tags, such as `">=1"` or `"1.*"`. The lint
skips those versions.

The lint skips a virtual workspace manifest because Cargo compiles no crate
from it.

The lint marks the suggestion as possibly incorrect, so `cargo fix` does not
apply it.

## Example

```toml
[dependencies]
anyhow = { version = "1.2" }
serde = "1"
```

## Use instead

```toml
[dependencies]
anyhow = { version = "1.2.0" }
serde = "1.0.0"
```
