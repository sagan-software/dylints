# missing_clippy_toml

## What it does

Checks that a `clippy.toml` file exists in the crate directory or in a parent
directory up to the workspace root. The workspace root is the nearest ancestor
whose `Cargo.toml` has a `[workspace]` table.

## Why is this bad?

Without a checked-in `clippy.toml`, Clippy uses its defaults or a
configuration file outside the repository. Different machines and CI can then
report different Clippy results for the same code.

## Known problems

The warning points at the crate root source file.

Only the exact name `clippy.toml` counts. A `.clippy.toml` file or a
`CLIPPY_CONF_DIR` setting, both of which Clippy accepts, does not satisfy the
lint. A `clippy.toml` above the workspace root is ignored.

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
