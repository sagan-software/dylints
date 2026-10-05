# AGENTS.md

## Downstream setup documentation

When adding these lints to another project or editing setup documentation,
read the [quick start](README.md#quick-start) and [setup guide](docs/usage.md).
Use Dylint's `workspace.metadata.dylint.libraries` in the target workspace's
root `Cargo.toml` or `dylint.toml`. Keep existing library entries.

Use `branch = "main"` and select library paths with `pattern`. The quick
start selects `lints/correctness`, `lints/perf`, and `lints/suspicious`.
`lints/*` selects all top-level groups, including stricter policy checks.
These settings are not application dependencies.

Install the compiler and Dylint tools shown in the quick start, then run
`cargo dylint --all --workspace -- --all-targets` in the target project.
Verify an expected diagnostic and a passing example before claiming setup
works. Keep Clippy and project tests as separate checks.

The lint repository configures its own linker. Keep the quick start free of
linker exports and build-directory commands. Document custom build settings
only under setup errors. Offer the Nix bundled runner as a separate setup path.

When changing setup instructions, verify `rust-toolchain.toml` and the Dylint
version, then test the copied settings in a separate project. State compiler
compatibility limits. Keep the full group hierarchy in the README, including
crate groups. Preserve the distinction between downstream Nix use and this
repository's Nix development workflow.

`CLAUDE.md` is a relative symlink to `AGENTS.md`. Edit this file to keep both
agent entry points in sync.

## Development workflow

Run Cargo commands inside `nix develop`. The shell supplies the pinned nightly
toolchain, `cargo-dylint`, `dylint-link`, a prebuilt Dylint driver, and the
`dylint-link` linker setting that UI tests require.

Use these commands:

- `cargo fmt --all` formats Rust source.
- `cargo clippy --workspace --lib --bins --tests -- -D warnings` runs the
  Clippy gate.
- `cargo test --workspace --lib --bins --tests` runs every unit, UI, and
  integration test.
- `cargo test -p LINT_NAME --lib` runs one lint's UI test.
- `cargo test -p sagan-lints --test runner` runs the runner integration tests.
- `nix run .#site` generates the lint catalog in `public/`.
- `nix flake check` builds the Nix packages and checks formatting.

## Dependency management

- Keep Rust dependencies in `Cargo.toml` and `Cargo.lock`.
- Keep toolchain and packaging dependencies in `flake.nix` and
  `nix/default.nix`.
- Prefer `inputs.<dep>.inputs.nixpkgs.follows = "nixpkgs"` for flake inputs.

## Creating Dylint lints

- Put lints under `lints/<category>/<lint_name>/`, or under
  `lints/crates/<crate>/<lint-name>/` for crate-specific lints. Add the crate to
  the root `Cargo.toml` `workspace.members`.
- Categories are Dylint group crates modeled on Clippy group names: `cargo`,
  `complexity`, `correctness`, `crates`, `maintainability`, `perf`,
  `restriction`, `style`, and `suspicious`. Add each new lint to its category
  crate's `Cargo.toml` dependencies and `src/lib.rs` `register_lints` calls.
- Keep category discovery intact: root `dylint.toml` and root `Cargo.toml`
  `[workspace.metadata.dylint]` list the category crate paths.
- When adding a category, also update `src/category.rs`, the category map in
  `nix/default.nix`, and `LintCategory` in `web/src/main.rs`.
- Declare every lint with the `Warn` default level.
- Model new lint crates on `lints/perf/ownership_at_boundaries` and
  `lints/suspicious/string_error_result`.

Each lint crate must contain this layout:

```text
lints/<category>/<lint_name>/
  Cargo.toml
  README.md
  src/lib.rs
  ui/main.rs
  ui/main.stderr
```

Use this `Cargo.toml` shape unless the lint needs more dependencies:

```toml
[package]
name = "<lint_name>"
version.workspace = true
description = "A lint to check for ..."
edition.workspace = true
publish.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
dylint_linting.workspace = true

[dev-dependencies]
dylint_testing.workspace = true

[features]
rlib = ["dylint_linting/constituent"]

[lints]
workspace = true

[package.metadata.rust-analyzer]
rustc_private = true
```

`src/lib.rs` requirements:

- Start with `#![feature(rustc_private)]` and
  `#![warn(unused_extern_crates)]`.
- Import the required `rustc_*` crates with `extern crate`.
- Declare the lint with `dylint_linting::declare_late_lint!` for HIR and type
  checks or `declare_early_lint!` for AST-only checks. Use `impl_late_lint!` or
  `impl_early_lint!` when the pass needs explicit state or a non-default
  constructor.
- Include Dylint-style docs in the declaration: `What it does`,
  `Why is this bad?`, `Known problems`, `Example`, and `Use instead`.
- Prefer semantic, type-aware rustc checks when a lint reasons about Rust
  types, traits, functions, methods, fields, derives, attributes, or resolved
  APIs. Use `LateContext`, `typeck_results`, `tcx.type_of`, `qpath_res`,
  `impl_trait_ref`, diagnostic items, or resolved `DefId`s instead of source
  spelling or raw text. Use AST and source-shape checks only when the subject is
  file text, item order, formatting, or unresolved syntax, and document why.
- Add this entry point for the UI test:

```rust
#[test]
fn ui() {
    dylint_testing::ui_test(env!("CARGO_PKG_NAME"), "ui");
}
```

Use `dylint_testing::ui_test_example` or `ui_test_examples` only when the UI
case must be a Cargo example target with its own dependencies.

`README.md` requirements:

- Use `# <lint_name>` as the title. The title must match the crate directory.
- Use these level-two sections in order: `What it does`, `Why is this bad?`,
  `Known problems`, `Example`, and `Use instead`. The catalog generator rejects
  any other structure.
- Keep examples small and representative of the UI tests.

UI test requirements:

- Put focused triggering and non-triggering examples in `ui/main.rs`.
- Put expected diagnostics in `ui/main.stderr`.
- Match the style of Dylint and compiletest with `$DIR` paths and `LL` line
  markers.
- When diagnostics change, run the tests and use the reported
  `Actual stderr saved to PATH` file to update `ui/main.stderr`. Do not write
  large stderr blocks by hand.

Before considering a new lint done:

1. Run `cargo fmt --all`.
2. Run `cargo test -p LINT_NAME --lib`.
3. Run the Clippy gate.
4. Run `cargo run --bin sagan-lints -- --list-private-lints` and confirm that
   the lint appears.
5. Run `nix run .#site` and confirm that generation succeeds.
