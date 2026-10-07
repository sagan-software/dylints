# Using Dylints

Dylints provides 297 compiler lints in nine general groups and 13
crate-specific groups. Select a group from the workspace root of the project
you want to check. The full category descriptions and crate links are in the
[README](../README.md#lint-groups).

## Compiler and tool requirements

Verified **2026-10-06** with Dylint **6.0.3** and the repository's
`nightly-2026-07-15` toolchain. The `cargo-dylint` and `dylint-link` command-line
tools can be installed with an ordinary Cargo toolchain; those executables do
not require nightly. The compiler-based lint libraries, this repository's
Dylint driver, and the target check use Rust's unstable `rustc_private` APIs.
The [Rust Unstable Book](https://doc.rust-lang.org/unstable-book/language-features/rustc-private.html)
requires `rustc-dev` and `llvm-tools` for official toolchains. Dylint builds the
target with the toolchain used to build the lint library, as described in the
versioned [Dylint compiler workflow](https://github.com/trailofbits/dylint/blob/v6.0.3/docs/how_dylint_works.md).
For this Dylints release, the compiler-facing components were built and tested
with `nightly-2026-07-15`; other compiler versions are not verified. A target
project may keep stable for ordinary builds, but its source and dependencies
must also compile when Dylint rechecks them with that nightly.

`rust-src`, Clippy, rustfmt and rust-analyzer are present in this repository's
pinned development toolchain for its driver and contributor workflows. Only
`rustc-dev` and LLVM tools are fundamental to linking `rustc_private` crates;
the other components serve the repository's configured build or optional
development tools.

On Linux, building Dylint's Git-backed libraries and driver needs a C compiler
and linker (`cc`), `pkg-config`, OpenSSL development files and zlib. Dylint
6.0.3 enables Git support through `git2`; its libgit2 build uses OpenSSL for
HTTPS and zlib for compression, while `pkg-config` locates the system headers
and libraries. See the upstream [Dylint workspace manifest](https://github.com/trailofbits/dylint/blob/v6.0.3/Cargo.toml)
and [`libgit2-sys` manifest](https://github.com/rust-lang/git2-rs/blob/master/libgit2-sys/Cargo.toml).
The repository development shell supplies these native tools. On macOS, install the Xcode
Command Line Tools and the system libraries needed by the project and Dylint.
This workflow was verified on x86_64 Linux; the repository's Nix development
flake also defines aarch64 Linux and aarch64 macOS environments. Windows
consumer setup has not been verified here.

### Install Dylint

See the [upstream Dylint installation guide](https://github.com/trailofbits/dylint/tree/v6.0.3#quick-start).
For the tested versions:

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview
cargo install --locked --version 6.0.3 cargo-dylint dylint-link
```

The install command uses your default Cargo toolchain. Use
`cargo +nightly-2026-07-15 dylint ...` for the compiler-facing check so the
target, lint libraries and Dylint driver use the tested compiler.

## Method 1: persist discovery in the target workspace's `Cargo.toml`

In the **root `Cargo.toml` of the workspace being checked**, add a Dylint
discovery table. Keep existing entries in `libraries`; this example chooses the
recommended general groups:

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
branch = "main"
pattern = ["crates/correctness", "crates/perf", "crates/suspicious"]
```

From that workspace root, ask Dylint to load the configured libraries and
check all targets:

```sh
cargo +nightly-2026-07-15 dylint --all --workspace -- --all-targets
```

To select only Bevy lints, use `pattern = "crates/bevy"` in the metadata
entry. This group checks Bevy-specific ECS, systems, schedules and engine API
patterns. It does not also load the general groups or the `crates` parent.

## Method 2: run Dylint directly for Bevy

To try the Bevy group without editing `Cargo.toml`, run this command from the
target workspace root:

```sh
cargo +nightly-2026-07-15 dylint \
    --git https://github.com/sagan-software/dylints \
    --branch main \
    --pattern crates/bevy \
    --workspace -- \
    --all-targets
```

`--git` identifies the lint repository and `--pattern` selects its Bevy group.
For reproducible builds, replace `--branch main` with `--tag TAG` or `--rev
COMMIT`. The Bevy group registers only Bevy lints; selecting it does not load
unrelated groups. Do not load a parent and any child groups together.

Both methods use ordinary Cargo and Dylint. The repository's `nix --accept-flake-config develop`
shell is a reproducible environment for contributors; downstream projects can
use their own shell or system packages. Downstream Nix environments need the
same native build tools and libraries described above. The repository's Nix
linker setup and build-directory configuration are development details; they
are not required in a consumer project's `Cargo.toml`.

## Groups and configuration

The nine general groups are `cargo`, `complexity`, `correctness`, `crates`,
`maintainability`, `perf`, `restriction`, `style` and `suspicious`. The
`crates` group contains 13 groups for Axum, Bevy, Clap, Insta, reqwest,
Schemars, Serde, SQLx, Strum, test-case, thiserror, Tokio and tracing. See the
[README's full group hierarchy](../README.md#lint-groups) for each group's
directory and general group scope. The crate-specific checks are:

- Axum checks router paths, nesting and service configuration for the [Axum web framework](https://github.com/tokio-rs/axum).
- Bevy checks ECS queries, systems, schedules and selected engine API usage in [Bevy](https://bevy.org/).
- Clap checks derive attributes and command-line argument configuration for [Clap](https://docs.rs/clap/latest/clap/).
- Insta checks snapshot assertions, filters and snapshot file handling in [Insta](https://insta.rs/).
- reqwest checks HTTP client construction, request loops, retries and TLS settings in [reqwest](https://github.com/seanmonstar/reqwest).
- Schemars checks derives and schema metadata for [Schemars](https://github.com/GREsau/schemars), a Rust library for generating JSON Schema.
- Serde checks serialization and deserialization attributes and round-trip behavior in [Serde](https://serde.rs/).
- SQLx checks query-builder use, row access and connection-pool settings in [SQLx](https://github.com/launchbadge/sqlx).
- Strum checks enum representation and derive behavior in [Strum](https://github.com/Peternator7/strum).
- test-case checks parameterized test declarations and case matrices in [test-case](https://github.com/frondeus/test-case).
- thiserror checks error derives, source fields and display formatting in [thiserror](https://github.com/dtolnay/thiserror).
- Tokio checks runtime, task, channel and blocking-call patterns in [Tokio](https://tokio.rs/).
- tracing checks spans, fields and instrumentation in [tracing](https://github.com/tokio-rs/tracing).

Configure lint options in a `dylint.toml` at the target workspace root.
Dylint config tables are keyed by the library's package name, including its
canonical kebab-case spelling. The repository's `dylint.toml` records every
option and its accepted values. The `rumdl_doc_comments` lint uses
`rumdl.toml` for Markdown rule settings.

Run `cargo dylint list --all` to inspect the registered lint names. Arguments
after `--` select Cargo targets, for example:

```sh
cargo +nightly-2026-07-15 dylint --all --workspace -- --lib --bins --tests
```

`DYLINT_RUSTFLAGS="-D warnings"` rejects lint warnings. Prefer a reasoned
`#[expect(...)]` for a reviewed exception; expectations flag exceptions that no
longer trigger. The catalog links each lint to its implementation and tests.
