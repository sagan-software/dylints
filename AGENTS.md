# AGENTS.md

## Downstream setup documentation

When adding these lints to another project or editing setup documentation,
read the [quick start](README.md#quick-start) and [setup guide](docs/usage.md).
Use Dylint's `workspace.metadata.dylint.libraries` in the target workspace's
root `Cargo.toml` or `dylint.toml`. Keep existing library entries.

Use `branch = "main"` and select library paths with `pattern`. The quick
start selects `crates/correctness`, `crates/perf`, and `crates/suspicious`.
The `crates/sagan-lints` aggregate selects all groups, including stricter policy checks.
These settings are not application dependencies.

Install the compiler and Dylint tools shown in the quick start, then run
`cargo dylint --all --workspace -- --all-targets` in the target project.
Verify an expected diagnostic and a passing example before claiming setup
works. Keep Clippy and project tests as separate checks.

The lint repository configures its own linker. Keep the quick start free of
linker exports and build-directory commands. Document custom build settings
only under setup errors. Document downstream Nix environments separately.

When changing setup instructions, verify `rust-toolchain.toml` and the Dylint
version, then test the copied settings in a separate project. State compiler
compatibility limits. Keep the full group hierarchy in the README, including
crate groups. Preserve the distinction between downstream Nix use and this
repository's Nix development workflow.

`CLAUDE.md` is a relative symlink to `AGENTS.md`. Edit this file to keep both
agent entry points in sync.

## Development workflow

Run Cargo commands inside `nix --accept-flake-config develop`. The shell supplies the pinned nightly
toolchain, `cargo-dylint`, `dylint-link`, a prebuilt Dylint driver, and the
`dylint-link` linker setting that UI tests require. It disables Rustup auto-install
because the pinned toolchain is read-only in `/nix/store`; this lets Dylint resolve
the active compiler without trying to mutate or download components into the store.
Nix command wrappers also include `cc` for `dylint-link` and GNU Make for the
Rumdl lint's native jemalloc dependency.

The workspace caps Cargo builds at one compiler job and test execution at one
thread. This flake requests one Nix derivation and one build core where the Nix
client controls scheduling. Those limits cap one invocation; a multi-user Nix
daemon can still receive work from other clients. Only the daemon's
system-wide configuration caps aggregate Nix build load. Avoid overlapping
heavyweight builds until an administrator sets `max-jobs` and `cores` in the
daemon configuration. Nix prompts before applying this flake's settings; pass
`--accept-flake-config` to each command that evaluates it.
Larger machines can raise Cargo's cap with `CARGO_BUILD_JOBS` and override Nix's
limits with `--max-jobs` and `--cores`.

Use these commands:

- `cargo fmt --all` formats Rust source.
- `nix --accept-flake-config fmt` formats Rust, Nix, and TOML files through treefmt and Taplo.
- `cargo clippy --workspace --lib --bins --tests -- -D warnings -A unknown-lints`
  runs the Clippy gate. Plain Cargo does not load Dylint libraries; the separate
  Dylint gate remains strict for custom lint names.
- `cargo nextest run --workspace --lib --bins --tests` runs test binaries using
  `.config/nextest.toml`.
- `cargo test --workspace --lib --bins --tests` runs every unit, UI, and
  integration test.
- `cargo test --workspace --doc` runs documentation tests, which nextest does
  not support.
- `cargo xtask coverage --min-lines 97 --min-file-lines 90 -- --workspace --lib --bins --tests` enforces aggregate and per-file executable-line coverage.
- `cargo test -p LINT_NAME --lib` runs one lint's UI test.
- `cargo test -p xtask --tests` runs development-command integration tests.
- `nix --accept-flake-config run .#site` generates the lint catalog in `public/`.
- `nix --accept-flake-config --max-jobs 1 --cores 1 flake check` builds the Nix packages and checks formatting with bounded parallelism.

## Dependency management

- Keep Rust dependencies in `Cargo.toml` and `Cargo.lock`.
- Keep toolchain and packaging dependencies in `flake.nix`.
- Prefer `inputs.<dep>.inputs.nixpkgs.follows = "nixpkgs"` for flake inputs.

## Creating Dylint lints

- Put each lint crate directly under `crates/<lint-name>/`. Put its category
  and crate-family groups beside it under `crates/<group>/`. Set the lint's
  category in `[package.metadata.dylint]`; do not nest lint crates under group
  directories. Keep `xtask` at the workspace root.
- Add the crate to root `Cargo.toml` workspace dependencies and its parent
  group's manifest and `register_lints` calls. Verify that `crates/*` includes
  it and Dylint discovery still lists only parent groups.
- Categories are Dylint group crates modeled on Clippy group names: `cargo`,
  `complexity`, `correctness`, `crates`, `maintainability`, `perf`,
  `restriction`, `style`, and `suspicious`. Add each new lint to its category
  crate's `Cargo.toml` dependencies and `src/lib.rs` `register_lints` calls.
- Keep category discovery intact: root `Cargo.toml`
  `[workspace.metadata.dylint]` lists the category crate paths. Keep lint options
  in `dylint.toml`; Dylint rejects duplicate discovery tables.
- When adding a category, also update discovery and category mapping in
  `flake.nix` and `crates/sagan-lints-web/src/category.rs`.
- Declare every lint with the `Warn` default level.
- Model new lint crates on `crates/ownership-at-boundaries` and
  `crates/string-error-result`.

Each lint crate must contain this layout:

```text
crates/<lint_name>/
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
  `Known problems`, `Example`, and `Use instead`. The catalog generator requires
  these sections first; additional interpretation and sources may follow.
- Keep examples small and representative of the UI tests.

UI test requirements:

- Put focused triggering and non-triggering examples in `ui/main.rs`.
- Put expected diagnostics in `ui/main.stderr`.
- Match the style of Dylint and compiletest with `$DIR` paths and `LL` line
  markers.
- When diagnostics change, run the tests and use the reported
  `Actual stderr saved to PATH` file to update `ui/main.stderr`. Do not write
  large stderr blocks by hand.
- Declare UI fixture examples with `test = false`; the `ui_test` harness builds
  them as examples, so Cargo should not schedule duplicate example test targets.

Before considering a new lint done:

1. Run `cargo fmt --all`.
2. Run `cargo test -p LINT_NAME --lib`.
3. Run the Clippy gate.
4. Run `cargo dylint list --all` and confirm that
   the lint appears.
5. Run `nix --accept-flake-config run .#site` and confirm that generation succeeds.

## Workspace and releases

The root is a virtual workspace. Use native Dylint discovery and `cargo xtask`;
there is no portable runner. The optional `lints` library aggregates categories.
Do not discover parent groups and their constituent libraries together.

Increment the shared workspace semver for every push. Use a patch
for compatible repairs, a minor for compatible additions, and a major for
breaking changes. Before 1.0, a minor increment marks a breaking change.
CI creates immutable tags and releases only after every validation and report
gate passes. Keep maintenance branches at `release/MAJOR` and
`release/MAJOR.MINOR`; do not move existing maintenance branches automatically.

Use reasoned `expect` attributes. Keep dependency sources in workspace
configuration and features with consumers. Replace standalone shell scripts
with tested Rust xtask commands. Preserve deliberate UI fixture violations.
