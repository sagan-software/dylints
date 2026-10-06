# Sagan Lints

Rust compiler lints loaded through [Dylint](https://github.com/trailofbits/dylint). General categories and crate-specific groups provide type-aware diagnostics. The catalog documents each lint, its limits and examples.

[Catalog](https://sagan-software.github.io/dylints/) · [Guide](https://sagan-software.github.io/dylints/book/) · [Coverage](https://sagan-software.github.io/dylints/coverage/) · [Benchmarks](https://sagan-software.github.io/dylints/benches/report/)

## Quick start

These lints use `nightly-2026-07-15`. Your project and its dependencies must build with that compiler. Your usual build can retain its current toolchain. Linux and macOS need a native linker such as `cc`. Linux also needs `pkg-config` and OpenSSL development libraries when Dylint builds its driver.

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview --component rust-src
cargo +nightly-2026-07-15 install --locked --version 6.0.3 cargo-dylint dylint-link
```

Add Git library discovery to your workspace root. Keep existing library entries:

```toml
[workspace.metadata.dylint]
libraries = [
    { git = "https://github.com/sagan-software/dylints", branch = "main", pattern = ["lints/correctness", "lints/perf", "lints/suspicious"] },
]
```

```sh
cargo +nightly-2026-07-15 dylint --all --workspace -- --all-targets
```

Select another category by changing `pattern`. The `lints/crates` group registers every nested crate-specific group. Listing nested groups alongside their parent registers lints twice. The optional `lints` aggregate registers every category; use it instead of separate category entries.

For reproducible use, replace `branch` with a validated release `tag` or exact commit `rev`. Dylint libraries depend on compiler internals and must match the selected nightly. These Git-loaded libraries use `publish = false`; a crates.io package cannot make a compiler-specific binary portable across Rust toolchains.

## Lint groups

Each group has its own library. Use its path in the `pattern` setting.
The full hierarchy is listed below. A parent group includes its child groups.
For example, `lints/crates/bevy` selects only the Bevy group.
Select a parent or its children; loading both can register a lint twice.

- [`cargo`](lints/cargo): Cargo and Rust project settings.
- [`complexity`](lints/complexity): code that can become simpler.
- [`correctness`](lints/correctness): likely bugs.
- [`crates`](lints/crates): checks for specific crate APIs.
  - [`axum`](lints/crates/axum).
  - [`bevy`](lints/crates/bevy).
  - [`clap`](lints/crates/clap).
  - [`insta`](lints/crates/insta).
  - [`reqwest`](lints/crates/reqwest).
  - [`schemars`](lints/crates/schemars).
  - [`serde`](lints/crates/serde).
  - [`sqlx`](lints/crates/sqlx).
  - [`strum`](lints/crates/strum).
  - [`test-case`](lints/crates/test-case).
  - [`thiserror`](lints/crates/thiserror).
  - [`tokio`](lints/crates/tokio).
  - [`tracing`](lints/crates/tracing).
- [`maintainability`](lints/maintainability): limits on code complexity and
  dependencies between parts of the code.
- [`perf`](lints/perf): needless allocation, copying, and repeated work.
- [`restriction`](lints/restriction): stricter rules that may not fit every project.
- [`style`](lints/style): Rust conventions and code layout.
- [`suspicious`](lints/suspicious): code and error handling worth checking.

Start with the quick-start groups and add others that fit your project.
Read each lint's known limits before adopting a strict group.

## Development

```sh
nix develop
cargo xtask --help
```

The shell provides the compiler, Dylint tools, a prebuilt driver and mdBook. See [contributing](docs/development.md), [performance](docs/performance.md) and [metric interpretation](docs/metrics.md).

The former portable runner and its profiles have been removed. Use native Dylint library discovery and Cargo target selection.

## License notes

The catalog's `web/static/` assets and `web/templates/index.html.jinja` adapt
the Clippy lint list. Clippy releases those assets under MIT or Apache-2.0.
See [the Clippy MIT license](web/static/LICENSE-CLIPPY-MIT).
