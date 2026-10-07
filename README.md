<p align="center">
  <img src="docs/assets/logo.svg" alt="Dylints mark" width="88" height="88">
</p>

# Dylints

[![CI](https://github.com/sagan-software/dylints/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/sagan-software/dylints/actions/workflows/ci.yml)
[![GitHub stars](https://img.shields.io/github/stars/sagan-software/dylints?style=flat)](https://github.com/sagan-software/dylints/stargazers)

Compiler lints for Rust rules that are hard to enforce consistently in review.
Dylints adds deterministic, type-aware checks for APIs, project policy and
maintenance patterns that otherwise depend on a reviewer remembering every
edge case. That makes the checks useful for human and AI-assisted changes alike.
They complement compiler diagnostics, Clippy, tests and code review.

The catalog contains **297 lints** in nine general groups and 13
crate-specific groups. Every lint has examples, known limits and a default
`Warn` level. The optional `crates/sagan-lints` aggregate selects every group,
including the stricter policy checks.

[Lint catalog](https://sagan-software.github.io/dylints/) ·
[Guide](https://sagan-software.github.io/dylints/book/) ·
[Coverage](https://sagan-software.github.io/dylints/coverage/) ·
[Benchmarks](https://sagan-software.github.io/dylints/benches/report/)

## Quick start

Install Dylint's command-line tools using the
[upstream installation guide](https://github.com/trailofbits/dylint#quick-start).
The examples below use Dylint 6.0.3 and the compiler used to validate this
repository, `nightly-2026-07-15`:

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview
cargo install --locked --version 6.0.3 cargo-dylint dylint-link
```

Install the CLI tools with your normal Cargo toolchain. The lint libraries and
Dylint driver use rustc's unstable compiler interfaces, so run Dylint with the
pinned nightly shown above. The target workspace and its dependencies must
compile under that nightly when Dylint checks them; ordinary application builds
can continue using stable Rust. The repository development environment and
compatibility details are in the [usage guide](docs/usage.md).

### Workspace metadata in `Cargo.toml`

In the **root `Cargo.toml` of the workspace you want to lint**, add a library
entry under `workspace.metadata.dylint`. Keep any entries already there. This
example selects the three recommended general groups:

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
branch = "main"
pattern = ["crates/correctness", "crates/perf", "crates/suspicious"]
```

Run Dylint from that workspace root:

```sh
cargo +nightly-2026-07-15 dylint --all --workspace -- --all-targets
```

### Direct command for Bevy lints

To try only the Bevy-specific group without changing the workspace manifest,
run this from the target workspace:

```sh
cargo +nightly-2026-07-15 dylint \
    --git https://github.com/sagan-software/dylints \
    --branch main \
    --pattern crates/bevy \
    --workspace -- \
    --all-targets
```

Choose a Git `tag` or exact `rev` instead of `branch = "main"` for
reproducible builds. Do not load a parent group and its child libraries
together: that registers the same lints more than once.

## Lint groups

Each general group is a Dylint library. Its directory is selected through the
`pattern` field above. The `crates` group contains the crate-specific groups;
select the parent or a child, never both.

### General groups

- [`cargo`](crates/cargo) checks Cargo manifests, workspace dependencies and toolchain configuration.
- [`complexity`](crates/complexity) finds control flow and iteration that can be simplified.
- [`correctness`](crates/correctness) catches likely bugs and failure-prone behavior.
- [`crates`](crates/crates) registers the crate-specific groups described below.
- [`maintainability`](crates/maintainability) measures complexity, coupling and public surface size.
- [`perf`](crates/perf) flags unnecessary allocations, copies and repeated work.
- [`restriction`](crates/restriction) enforces stricter project policies that may not suit every codebase.
- [`style`](crates/style) checks Rust conventions and consistent implementations.
- [`suspicious`](crates/suspicious) highlights error handling and constructs that deserve review.

### Crate-specific groups

Each linked name is the crate's canonical spelling and points to its official
project or documentation.

- [`axum`](crates/axum) checks router paths, nesting and service configuration for the [Axum web framework](https://github.com/tokio-rs/axum).
- [`bevy`](crates/bevy) checks ECS queries, systems, schedules and selected engine API usage in [Bevy](https://bevy.org/).
- [`clap`](crates/clap) checks derive attributes and command-line argument configuration for [Clap](https://docs.rs/clap/latest/clap/).
- [`insta`](crates/insta) checks snapshot assertions, filters and snapshot file handling in [Insta](https://insta.rs/).
- [`reqwest`](crates/reqwest-lints) checks HTTP client construction, request loops, retries and TLS settings in [reqwest](https://github.com/seanmonstar/reqwest).
- [`schemars`](crates/schemars) checks derives and schema metadata for [Schemars](https://github.com/GREsau/schemars), a Rust library for generating JSON Schema.
- [`serde`](crates/serde) checks serialization and deserialization attributes and round-trip behavior in [Serde](https://serde.rs/).
- [`sqlx`](crates/sqlx) checks query-builder use, row access and connection-pool settings in [SQLx](https://github.com/launchbadge/sqlx).
- [`strum`](crates/strum) checks enum representation and derive behavior in [Strum](https://github.com/Peternator7/strum).
- [`test-case`](crates/test-case) checks parameterized test declarations and case matrices in [test-case](https://github.com/frondeus/test-case).
- [`thiserror`](crates/thiserror) checks error derives, source fields and display formatting in [thiserror](https://github.com/dtolnay/thiserror).
- [`tokio`](crates/tokio) checks runtime, task, channel and blocking-call patterns in [Tokio](https://tokio.rs/).
- [`tracing`](crates/tracing) checks spans, fields and instrumentation in [tracing](https://github.com/tokio-rs/tracing).

## Development

Use the pinned environment for repository development and its compiler-based
UI tests:

```sh
nix --accept-flake-config develop
cargo xtask --help
```

See [contributing](docs/development.md), [performance](docs/performance.md)
and [metric interpretation](docs/metrics.md) for validation and report details.
The catalog and guide are built as one Pages artifact by `cargo xtask site`.

## License notes

The catalog's `crates/sagan-lints-web/static/` assets and
`crates/sagan-lints-web/templates/index.html.jinja` adapt the Clippy lint list.
Clippy releases those assets under MIT or Apache-2.0. See [the Clippy MIT
license](crates/sagan-lints-web/static/LICENSE-CLIPPY-MIT).
