# rust_toolchain_toml

## What it does

Checks for a file named `rust-toolchain`, without the `.toml` extension, in
the crate root file's directory or any parent directory. The warning points at
the start of that file.

## Why is this bad?

The bare `rust-toolchain` name is the legacy rustup form. The name does not
show whether the file holds a one-line channel or TOML, and editors and other
tools do not treat it as TOML.

## Known problems

The search continues to the file system root, so a `rust-toolchain` file
outside the repository also triggers the lint. The lint warns even when a
`rust-toolchain.toml` file exists next to the bare file.

When the file is not valid UTF-8, the warning points at the crate root source
file instead.

## Example

```text
my-crate/
├── Cargo.toml
├── rust-toolchain
└── src/lib.rs
```

## Use instead

```text
my-crate/
├── Cargo.toml
├── rust-toolchain.toml
└── src/lib.rs
```
