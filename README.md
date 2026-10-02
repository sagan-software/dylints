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

The README code blocks are lint examples, not complete programs, so the test
command skips doctests.
UI tests marked `// run-rustfix` apply machine-applicable suggestions and
compile the resulting program. Their `.fixed` files record the expected rewrite.

Run one lint's UI tests:

```sh
cargo test -p ad_hoc_display --lib
```

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
`sagan-lints --list-private-lints`. A lint is listed as `MachineApplicable` when
its source, or a support macro it invokes, emits a machine-applicable
suggestion and its UI tests render one. Every other lint is listed as
`NotMachineApplicable`. The CI workflow deploys the catalog to GitHub Pages
from `main`.

## CI

[`.github/workflows/ci.yml`](.github/workflows/ci.yml) runs formatting, Clippy,
and the test suite on every push and pull request. It also builds the catalog
and runs the bundled lints against this repository. The self-lint job does not
block merges until the repository passes its own lints.

## License notes

`web/static/` and `web/templates/index.html` adapt the Clippy lint list, which
is licensed under MIT or Apache-2.0. See
[`web/static/LICENSE-CLIPPY-MIT`](web/static/LICENSE-CLIPPY-MIT).
