# Using the lints

Install `cargo-dylint` and `dylint-link` 6.0.3 and select `nightly-2026-07-15`, including `rustc-dev`, `llvm-tools-preview` and `rust-src`. The repository's `nix develop` shell provides these tools.

Use the [installation commands](../README.md#quick-start) before configuring discovery. A downstream project can use its own Nix shell for system dependencies while selecting this pinned compiler. The repository development shell serves contributors; it is not a portable project runner.

A downstream Nix shell needs `pkg-config` and OpenSSL development libraries for Dylint's driver, in addition to the project's system dependencies.

## Workspace discovery

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/sagan-software/dylints", branch = "main", pattern = ["lints/correctness", "lints/perf", "lints/suspicious"] },
]
```

```sh
cargo +nightly-2026-07-15 dylint --all --workspace -- --all-targets
```

The available general groups are `cargo`, `complexity`, `correctness`, `crates`, `maintainability`, `perf`, `restriction`, `style` and `suspicious`. The `crates` group includes Bevy, Axum, Clap, Serde and other crate-specific groups. Use the optional `lints` aggregate to load every group through one library. Do not load an aggregate and its constituents together.

Pin `tag` or `rev` for reproducible builds. A library's compiler version must match the target's compiler. Dylint 6.0.3 supports Git sources, patterns, tags and revisions through [workspace metadata](https://github.com/trailofbits/dylint/blob/v6.0.3/README.md).

## Commands and configuration

```sh
cargo dylint list --all
cargo dylint --all --workspace -- --lib --bins --tests
```

Arguments after `--` select Cargo targets. `DYLINT_RUSTFLAGS="-D warnings"` rejects lint warnings. Ordinary integration tests belong in that scope. UI and Cargo example fixtures that deliberately trigger diagnostics are separate test inputs.

Configure lint tables in `dylint.toml`. The repository file documents every configurable lint, its defaults and accepted values. `rumdl_doc_comments` uses `rumdl.toml` for Markdown rule settings.

Prefer reasoned `#[expect(...)]` for a reviewed exception. Expectations detect checks that no longer trigger. The catalog's source and documentation links identify each lint's implementation and tests.
