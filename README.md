# Sagan's Rust lints

![Sagan lints logo](docs/assets/logo.svg)

[![CI](https://github.com/sagan-software/dylints/actions/workflows/ci.yml/badge.svg)](https://github.com/sagan-software/dylints/actions/workflows/ci.yml)
[![Lint catalog](https://img.shields.io/badge/docs-lint_catalog-2563eb)](https://sagan-software.github.io/dylints/)
[![Dylint 6.0.3](https://img.shields.io/badge/Dylint-6.0.3-334155)](https://github.com/trailofbits/dylint/tree/v6.0.3)
[![Rust nightly](https://img.shields.io/badge/Rust-nightly--2026--07--15-334155)](rust-toolchain.toml)

Extra Rust checks for code, project settings, and crate APIs. Catch needless
copies, weak error handling, and common API mistakes while you build.

These lints run through [Dylint](https://github.com/trailofbits/dylint), a tool
that loads extra checks into the Rust compiler. A lint is a check that reports
a possible problem in your code. You choose which groups to run alongside
Clippy, Rust's usual lint tool.

[Browse all lints](https://sagan-software.github.io/dylints/) ·
[Setup guide](docs/usage.md) · [Contributing](docs/development.md)

## Contents

- [Quick start](#quick-start)
- [Features](#features)
- [Lint groups](#lint-groups)
- [Bundled runner](#bundled-runner)
- [Documentation](#documentation)
- [Development](#development)
- [License notes](#license-notes)

## Quick start

On Linux or macOS, you need a Rust project, [rustup](https://rustup.rs/), and a native linker
such as `cc`. You do not need to clone this repository or install Nix.

These lints use `nightly-2026-07-15`. Dylint checks your project with that
compiler, so your project and its dependencies must build with it. Your usual
build can keep using its current toolchain.

### 1. Install the tools

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview
cargo +nightly-2026-07-15 install --locked --version 6.0.3 cargo-dylint dylint-link
```

`cargo-dylint` runs the checks. `dylint-link` prepares the lint libraries for
loading. These versions match this repository's tested setup.

### 2. Add the lints to your project

Add this to your project's root `Cargo.toml`. In a workspace with several
crates, use the workspace's root file.

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
rev = "483b64d83e38352994d509eacf4a56db1892f1a3"
pattern = ["lints/correctness", "lints/perf", "lints/suspicious"]
```

This selects three groups and pins them to a known commit. If you already have
library entries, add this entry beside them. These are tool settings, not
application dependencies.

The same block can go in a root `dylint.toml` instead. Choose one file for the
entry. Dylint explains this format in its
[workspace metadata guide](https://github.com/trailofbits/dylint/tree/v6.0.3#workspace-metadata).

### 3. Run the checks

In the same terminal, select the lint linker for your machine:

```sh
unset CARGO_BUILD_BUILD_DIR
lint_host=$(rustc +nightly-2026-07-15 -vV | sed -n 's/^host: //p')
lint_host=$(printf '%s' "$lint_host" | tr '[:lower:]-' '[:upper:]_')
export "CARGO_TARGET_${lint_host}_LINKER=dylint-link"
```

This lets Dylint find the libraries it builds. It also clears a custom Cargo
build-directory setting that Dylint 6.0.3 cannot use. Repeat this setup in each
new terminal where you run Dylint. Your project's files stay unchanged.

Run this from your project's root:

```sh
cargo dylint --all --workspace -- --all-targets
```

Dylint downloads and builds the selected lint groups on the first run. Later
runs reuse the build. Diagnostics show the lint name and source location.
Every lint defaults to a warning.

`--all` loads all configured lint libraries. `--workspace` checks every project
crate. `--all-targets` also checks tests, examples, and benches.

To make lint warnings fail a CI check, use:

```sh
DYLINT_RUSTFLAGS="-D warnings" cargo dylint --all --workspace -- --all-targets
```

This also treats the compiler's other warnings as errors. Keep running
`cargo clippy` separately. See the [setup guide](docs/usage.md) for all groups,
one-lint setup, CI, editor use, and fixes for setup errors.

## Features

- Choose a whole group or a single lint through project settings.
- Check resolved Rust types and APIs where the rule needs that information.
- Read each lint's purpose, limits, and before-and-after examples in the
  [lint catalog](https://sagan-software.github.io/dylints/).
- Apply automatic fixes where a lint provides a compiler-approved suggestion.
  The catalog marks these as `MachineApplicable`.
- Use focused tests that check both code that triggers a lint and code that
  should pass. Some tests also compile the suggested fix.
- Run strict Clippy and the bundled lints together with `sagan-lints`.

For example, [`ownership_at_boundaries`](lints/perf/ownership_at_boundaries/README.md)
flags a public function that takes ownership of a vector but only reads it:

```rust
pub fn count_errors(lines: Vec<String>) -> usize {
    lines.iter().filter(|line| line.contains("ERROR")).count()
}
```

Accept a slice so callers can pass their data without copying it:

```rust
pub fn count_errors(lines: &[String]) -> usize {
    lines.iter().filter(|line| line.contains("ERROR")).count()
}
```

## Lint groups

Each group has its own library. Use its path in the `pattern` setting.

- [`cargo`](lints/cargo): Cargo and Rust project settings.
- [`complexity`](lints/complexity): code that can become simpler.
- [`correctness`](lints/correctness): likely bugs.
- [`crates`](lints/crates): checks for specific crate APIs, including Serde,
  Tokio, SQLx, Clap, and Reqwest.
- [`maintainability`](lints/maintainability): limits on code complexity and
  dependencies between parts of the code.
- [`perf`](lints/perf): needless allocation, copying, and repeated work.
- [`restriction`](lints/restriction): stricter rules that may not fit every project.
- [`style`](lints/style): Rust conventions and code layout.
- [`suspicious`](lints/suspicious): code and error handling worth checking.

Start with the quick-start groups and add others that fit your project.
Read each lint's known limits before adopting a strict group.

## Bundled runner

If you use [Nix](https://nixos.org/download/), you can run strict Clippy and
the bundled lints with one command, without adding Dylint settings:

```sh
nix run github:sagan-software/dylints -- --repo /path/to/your/project --fast
```

Nix needs flakes enabled. The package includes the compiler, lint libraries,
and driver. The first build can take time. The runner still needs access to
your project's dependencies and uses its pinned compiler.

The runner enforces more rules than the quick start, including strict Clippy.
It supports package and group selection, automatic fixes, and checks limited
to changed lines. See the [runner guide](docs/usage.md#bundled-runner).

## Documentation

- [Setup and use](docs/usage.md): select lints, pin versions, run CI and editor
  checks, and solve setup errors.
- [Lint catalog](https://sagan-software.github.io/dylints/): every lint's rule,
  examples, limits, and fix support.
- [Development](docs/development.md): build, test, coverage, and catalog generation.
- [Agent instructions](AGENTS.md): repository workflow and downstream setup
  guidance. `CLAUDE.md` is a symlink to this file.
- [Upstream Dylint](https://github.com/trailofbits/dylint): the tool that loads
  these lint libraries.

## Development

Clone the repository and enter its development shell:

```sh
git clone https://github.com/sagan-software/dylints.git
cd dylints
nix develop
```

The shell supplies the pinned compiler, Dylint tools, driver, and linker
settings. Run the repository checks inside it:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --lib --bins --tests -- -D warnings
cargo test --workspace --lib --bins --tests
```

See [development details](docs/development.md) for the repository layout,
coverage reports, and building the catalog.

## License notes

The catalog's `web/static/` assets and `web/templates/index.html.jinja` adapt
the Clippy lint list. Clippy releases those assets under MIT or Apache-2.0.
See [the Clippy MIT license](web/static/LICENSE-CLIPPY-MIT).
