# cargo

Rust review lints for Cargo and Rust project metadata.

## Lints

- [`dependency_full_semver_versions`](dependency_full_semver_versions): flags
  dependency version requirements like `"1"` or `"0.1"` that omit a patch
  part.
- [`dependency_key_order`](dependency_key_order): flags unsorted adjacent
  entries in Cargo dependency tables.
- [`missing_clippy_toml`](missing_clippy_toml): flags crates without a
  `clippy.toml` file in the crate directory or workspace root.
- [`missing_rust_toolchain_toml`](missing_rust_toolchain_toml): flags crates
  without a `rust-toolchain.toml` file in the crate directory or workspace root.
- [`package_lints_section`](package_lints_section): flags package manifests
  that define `[package]` without a package-level `[lints]` table.
- [`rust_toolchain_toml`](rust_toolchain_toml): flags bare `rust-toolchain` files; the expected name is `rust-toolchain.toml`.
- [`workspace_dependency_versions`](workspace_dependency_versions): flags
  workspace package dependencies that set a `version` instead of inheriting it
  from `[workspace.dependencies]`.
- [`workspace_lints_inheritance`](workspace_lints_inheritance): flags workspace
  package manifests that define `[lints]` without `workspace = true`.
