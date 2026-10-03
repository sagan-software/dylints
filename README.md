# Sagan's Rust lints

This repository holds Sagan's custom Rust lints for
[Dylint](https://github.com/trailofbits/dylint) and the `sagan-lints` runner that
bundles them. The [lint catalog](https://sagan-software.github.io/dylints/) lists
every lint with its documentation.

## Layout

- `lints/`: Dylint lint crates, grouped into category libraries.
- `src/`: the `sagan-lints` runner. It runs strict Clippy and the bundled lints
  against a target repository.
- `profiles/`: the Clippy and rustc lint lists that the runner enables.
- `support/`: shared helpers for lint crates and the runner build script.
- `web/`: the lint catalog generator, adapted from the
  [Clippy lint list](https://rust-lang.github.io/rust-clippy/stable/index.html).
- `nix/` and `flake.nix`: the pinned toolchain, Dylint tools, and packages.

## Lint categories

Every lint defaults to `warn`.

- [`cargo`](lints/cargo): Cargo and Rust project metadata.
- [`complexity`](lints/complexity): code that can usually become simpler.
- [`correctness`](lints/correctness): code that is likely to behave
  incorrectly.
- [`crates`](lints/crates): crate-specific misuse patterns from official crate
  docs.
- [`maintainability`](lints/maintainability): quantitative complexity and
  coupling limits.
- [`perf`](lints/perf): avoidable performance costs.
- [`restriction`](lints/restriction): local policy restrictions.
- [`style`](lints/style): idiomatic style.
- [`suspicious`](lints/suspicious): suspicious code and error handling.

## Development

The workspace uses the nightly toolchain pinned in `rust-toolchain.toml` with
the `rustc-dev` component. Dylint UI tests also need `cargo-dylint` and
`dylint-link` 6.0.3, with `dylint-link` as the target linker.

Enter the Nix development shell to get the toolchain, the Dylint tools, a
prebuilt Dylint driver, and the linker setting:

```sh
nix develop
```

Run the same gates as CI from that shell:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --lib --bins --tests -- -D warnings
cargo test --workspace --lib --bins --tests
```

The test command runs unit, UI, and integration tests. Some lint README examples
depend on application types, so the command skips doctests.
UI tests marked `// run-rustfix` apply machine-applicable suggestions and
compile the resulting program. Their `.fixed` files record the expected rewrite.

Run the same gates through the Nix check app:

```sh
nix run .#check
```

Run one lint's UI tests:

```sh
cargo test -p ad_hoc_display --lib
```

## Coverage

Measure the workspace through the Nix coverage app:

```sh
nix run .#coverage
```

The command builds fresh instrumented binaries and runs every workspace test.
It writes reports to `target/coverage/report`.
`canonical_summary.json` merges lexical paths that name the same source file.
`canonical_gaps.txt` lists executable lines with no hits.
`summary.json`, `summary.txt`, and `lcov.info` keep the LLVM reports.
`llvm-cov.stderr` keeps mapping warnings.

The line threshold uses canonical executable lines from production and test
targets, including `cfg(test)` modules. Reports exclude examples, UI fixtures,
and dependency sources. Region totals keep each compiled source mapping.
These metrics do not measure branch coverage.

To measure a lint's coverage with a minimum of 99%, run:

```sh
nix run .#coverage -- --min-lines 99 --path lints/style/ad_hoc_display -- -p ad_hoc_display --lib --tests
```

`--min-lines` accepts digits with an optional decimal fraction, from 0 through
100. Each `--path` must name an existing directory with at least one selected
Rust source. Symlinked Rust sources are supported. Invalid arguments fail
before the command builds or replaces reports.

Each run replaces the generated build, profiles, and reports in its coverage
directory. Set `COVERAGE_TARGET_DIR` to a dedicated directory to keep runs separate.
Coverage compiles the workspace again and needs extra disk space.

## Documentation checks

The runner uses `rumdl_doc_comments` to check documentation comments in Rust.
Standalone Markdown and prose need separate Markdown and writing tools.

## Runner

Run strict Clippy and the bundled lints against a Rust workspace:

```sh
cargo run --bin sagan-lints -- --repo /path/to/workspace
```

Run one crate or lint category, or list the bundled lints:

```sh
cargo run --bin sagan-lints -- --repo /path/to/workspace --package crate_name
cargo run --bin sagan-lints -- --repo /path/to/workspace --skip-clippy --dylint-category style
cargo run --bin sagan-lints -- --list-private-lints
```

The runner also supports `--fast`, `--fix`, `--target-dir PATH`, and
`--changed-range REVISION_RANGE`. The `--fix` mode applies only compiler
suggestions marked machine-applicable and reruns the selected checks.

The Nix package builds a self-contained runner:

```sh
nix build
nix run . -- --repo /path/to/workspace --fast
```

## Lint catalog

Generate the catalog into `public/`:

```sh
nix run .#site
```

The generator reads each lint's `README.md` and takes default levels from
`sagan-lints --list-private-lints`. The catalog labels a lint
`MachineApplicable` when
its source, or a support macro it invokes, emits a machine-applicable
suggestion and its UI tests render one. It labels every other lint
`NotMachineApplicable`. Category tests and nested auxiliary fixtures do not
create catalog entries. The CI workflow deploys the catalog to GitHub Pages
from `main`.

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs formatting, Clippy,
and tests for pushes to `main`, `pull_request` events, and manual runs. It also
builds the catalog, measures coverage, and runs the bundled lints against this
repository. The self-lint job fails when a bundled lint reports a finding.

## License notes

`web/static/` and `web/templates/index.html` adapt the Clippy lint list.
Clippy releases these assets under MIT or Apache-2.0. See
[`web/static/LICENSE-CLIPPY-MIT`](web/static/LICENSE-CLIPPY-MIT).
