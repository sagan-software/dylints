# dependency_full_semver_versions

## What it does

Checks the nearest `Cargo.toml` above the crate root file for dependency
versions that omit the patch part, such as `"1"` or `"0.1"`. It checks
`[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`, and
`[workspace.dependencies]`.

## Why is this bad?

Cargo reads `"1"` as `^1.0.0`, so it accepts every `1.x` release, including
releases older than the code needs. Reviewers cannot see which minimum version
the crate was tested with.

## Known problems

The warning points at the crate root source file because the lint has no span
in `Cargo.toml`. The message names the dependency and its version.

The lint only flags plain numeric versions with one or two parts. Versions with
operators, wildcards, or pre-release tags, such as `">=1"` or `"1.*"`, are not
checked.

Target-specific tables such as `[target.'cfg(unix)'.dependencies]` are not
checked.

The manifest is read line by line. Multiline inline tables, quoted dependency
names, and escaped strings can be skipped or misread.

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
