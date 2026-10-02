# missing_clippy_toml

## What it does

Checks for a `clippy.toml` or `.clippy.toml` file in the package directory or a
parent directory up to the workspace root. The workspace root is the package
manifest itself when it has a `[workspace]` table. Otherwise, it is the nearest
ancestor `Cargo.toml` with a `[workspace]` table that does not exclude the
package. The warning points at the `[package]` header of the manifest.

## Why is this bad?

Without a checked-in `clippy.toml`, Clippy uses its defaults or a
configuration file outside the repository. Different machines and CI can then
report different Clippy results for the same code.

## Known problems

Clippy accepts a `CLIPPY_CONF_DIR` setting, but the lint does not treat it as
satisfying. The lint ignores a `clippy.toml` above the workspace root. It does
not read a `package.workspace` key that points at the workspace root.

The lint skips crates without a package manifest, such as files compiled
directly with `rustc`.

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
