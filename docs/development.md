# Contributing

Run Cargo commands inside `nix --accept-flake-config develop`. Keep dependency sources in `[workspace.dependencies]`; consumers inherit sources and specify their own features. The virtual workspace contains category libraries, leaf lints, shared support, `web` and `xtask`. All workspace crates except `xtask` are direct children of `crates/`; the `crates/*` glob leaves nested UI fixtures outside workspace discovery. There is no portable runner.

The locked Nix development shell and CI use [cargo-nextest](https://nexte.st/docs/) 0.9.143 for compatible test binaries. Nextest does not execute rustdoc tests, so the exact Cargo doctest command below remains separate. The repository also keeps its exact Cargo unit, binary and integration-test gate.

Build and test concurrency is capped for workstation-sized machines. Cargo
uses one compiler process, and Cargo's test harness and nextest use one test
thread. The flake requests one Nix derivation and one build core where the Nix
client controls scheduling. Separate clients can still submit work
concurrently.

On a multi-user installation, the Nix daemon schedules builds for every client
and reads the system-wide configuration. A flake or command-line limit caps
that invocation; a per-user setting supplies defaults to that user's commands.
Neither caps combined Nix work submitted by other clients. This machine's daemon
currently has `max-jobs = auto` and no memory limit. Avoid overlapping
heavyweight builds until the daemon limit is changed.

To cap all local Nix builds on a multi-user installation, an administrator
must set `max-jobs = 1` and `cores = 1` in the Nix daemon's global
configuration and restart the daemon. On this machine, Determinate Nix marks
`/etc/nix/nix.conf` as managed and includes `/etc/nix/nix.custom.conf`; put the
settings in that included file instead of editing the managed file. The
daemon-wide cap affects every project on the machine. After existing builds
finish, an administrator can add these lines to `/etc/nix/nix.custom.conf`:

```ini
max-jobs = 1
cores = 1
```

Then restart the daemon:

```sh
sudo systemctl restart nix-daemon.service
```

The repository does not change system configuration. Per-user and per-flake
settings cap the invocations that use them, but do not replace the daemon-wide
cap across independent Nix clients. Standalone Cargo commands use this
workspace's one-job configuration; other projects need their own job cap
before compiling in parallel with Dylints.

Nix asks before applying this flake's settings; pass `--accept-flake-config` to
each Nix command that evaluates this flake. Raise limits only when memory is
available. These settings were verified with Nix 2.35.2 on 2026-10-06. Cargo's
[`build.jobs`](https://doc.rust-lang.org/cargo/reference/config.html#build) can
be overridden with `CARGO_BUILD_JOBS`; Nix's
[build limits](https://nix.dev/manual/nix/2.35/advanced-topics/cores-vs-jobs)
are controlled by the daemon on a multi-user installation; nextest's
[`test-threads`](https://nexte.st/docs/configuration/reference/) can be
overridden with `--test-threads`.

## Nix environment

`nix --accept-flake-config develop` reads the checked-in toolchain file through rust-overlay and uses Crane for cached Cargo builds. It sets `RUSTUP_AUTO_INSTALL=0` because the pinned compiler lives read-only in `/nix/store`; Dylint invokes `rustup show active-toolchain` while discovering libraries, and rustup must not try to install the file's components into that Nix store path. Nix command wrappers include the C compiler wrapper because `dylint-link` delegates compiler links to `cc`, and GNU Make because the Rumdl lint's jemalloc dependency builds native code. Dylint 6.0.3's [`driver` package](https://github.com/trailofbits/dylint/tree/v6.0.3/driver) exposes a library, so `nix/dylint-driver` supplies a small executable wrapper that Nix builds with the upstream lockfile. The development shell keeps Dylint's linker and target-directory setup for UI tests; downstream projects do not copy it. Native OpenSSL and zlib support Dylint's Git transport, as described in the [usage guide](usage.md#compiler-and-tool-requirements).

The review baseline was 315 lines in `flake.nix`; the current file is 245
lines, a 22.2% reduction. The 25% limit would be 236 lines. The remaining
`dylintTools`, driver-wrapper, shell-setup, `treefmt` and `workspaceChecks`
definitions provide the Dylint tools and executable, pinned compiler/linker,
formatter, and required CI gates. The nine-line gap is retained for build
limits: the flake's one-job/one-core defaults and explicit one-job Cargo caps.
Removing those settings weakens the workstation memory limits. The single
formatter and check outputs replace the duplicate local check wrappers;
`cargo xtask site` remains the site-generation path.

## Validation

```sh
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --lib --bins --tests -- -D warnings -A unknown-lints
cargo nextest run --workspace --lib --bins --tests
cargo test --workspace --lib --bins --tests
cargo test --workspace --doc
cargo xtask lint
cargo xtask coverage --min-lines 97 --min-file-lines 90 -- --workspace --lib --bins --tests
cargo xtask bench
cargo xtask site
nix --accept-flake-config fmt
nix --accept-flake-config --max-jobs 1 --cores 1 flake check
```

Before changing a lint, add a failing UI regression at its compiler boundary. Run `cargo test -p LINT_NAME --lib` after each coherent change. When diagnostics change, copy the test's reported actual stderr into the expected fixture. When fixture imports or suggested fixes change, copy the test's reported actual fixed output into the rustfix fixture. When a fixture deliberately violates a lint, preserve that input instead of applying its suggested fix. Verify triggering and non-triggering cases.

`cargo xtask lint` checks ordinary library, binary and integration-test targets. Deliberate UI and example fixtures are excluded. New production paths require direct tests; line hits alone do not prove correctness.

The Clippy gate allows only `unknown-lints` because ordinary Cargo does not load this workspace's Dylint libraries, while source crates use `#[expect]` for those custom lint names. The separate `cargo dylint` gate runs with `-D warnings`, so custom lint expectations and diagnostics remain strict there.

Coverage tests retain artifacts after test failure and reject unsafe output targets. They propagate LLVM export, summary-write and terminal-write failures. Site tests propagate catalog and book failures before report assembly.

The process fixture is a Cargo example with `clap` derive parsing. Tests build it in a private target directory. `cargo fmt --all` includes its source; `nix --accept-flake-config fmt` checks it with the rest of the repository.

## Adding a lint

Place each lint as a direct child of `crates/`, beside its category and crate-specific group crates. Its manifest inherits dependencies, package metadata and lint configuration. Declare its default level as `Warn`. Register it through its parent group's `rlib` constituent dependency. Parent groups register nested groups; discovery lists only parents.

The optional aggregate at `crates/sagan-lints` owns all nine general categories and must be discovered independently. Its UI fixture also records Cargo-policy diagnostics from the isolated compiler input.

Each lint owns `Cargo.toml`, `README.md`, `src/lib.rs`, `ui/main.rs` and `ui/main.stderr`. Keep the five required README sections in order: `What it does`, `Why is this bad?`, `Known problems`, `Example` and `Use instead`. Add interpretation and primary sources after those sections when needed.

Prefer resolved compiler types, methods and definitions for semantic checks. Source-layout lints can use AST structure when they explain that boundary. Shared compiler helpers live in `support`; category helpers remain with their category.

The compiler-sysroot bootstrap tests compile and execute the unchanged Cargo build script. They verify compiler selection, symlinks, PATH order, missing configuration, invalid layouts and protocol-write failure. Instrumented runs retain this executable in the coverage object archive, because Cargo build-script binaries are excluded from ordinary object discovery.

## Reports

`cargo xtask coverage` validates percentages and scope directories before replacing generated outputs. It instruments test binaries and lint libraries, merges profiles and writes canonical HTML, JSON, text, LCOV, TSV and gap reports. Each compiler object is exported separately, then source aliases merge by canonical path and line, retaining a hit when any compiled variant reaches that line. The HTML, JSON, LCOV data and threshold all show one entry per canonical source file; raw LLVM alias rows are not published. The aggregate floor is 97%; each source file shown in the report must also reach 90%. Inspect `canonical_gaps.txt` for exact zero-hit source lines; inspect assertions before treating a covered line as correct. Examples, UI files, fixture directories, and support crates whose directory name ends in `-fixture` are excluded; `cfg(test)` and ordinary integration tests are included.

`cargo nextest run --workspace --lib --bins --tests` consumes `.config/nextest.toml` for ordinary test binaries, including Dylint UI test harnesses. Nextest does not run Rust documentation tests; run `cargo test --workspace --doc` separately. The exact Cargo unit, binary and integration-test command remains a separate repository gate.

Constituent builds can replace toolchain-named lint libraries with another feature variant. A Unix linker wrapper retains each successfully linked shared library before replacement. It delegates unchanged arguments to `dylint-link`; compiler commands remain visible to Dylint's UI tests. Canonical coverage exports each retained object separately and merges line hits because combined LLVM mappings can discard another variant's counters. Retaining libraries increases coverage disk use; coverage replaces only its owned build, object, profile and report directories.

The catalog follows [rustdoc hidden-line rules](https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html): Rust setup lines beginning with a hash followed by a space, or a lone `#`, are hidden, and an escaped leading `##` displays one fewer hash. Other code languages retain their comments.

Runnable README examples compile as documentation tests. Deliberately invalid examples use `compile_fail`. Abbreviated file-layout examples and SQLx fixture sketches use `ignore` and explain their omitted context; documentation tests do not verify those sketches against PostgreSQL.

Use repeated `--path DIRECTORY` options for a focused report. Relative paths resolve from the workspace. `COVERAGE_TARGET_DIR` overrides the dedicated output directory. The workspace root, its ancestors and `target` itself are rejected. Coverage tests reject malformed input before cleanup and prove that owned generated outputs are replaced.

Failed tests remain failures even when coverage reaches the threshold. LLVM warnings remain in `llvm-cov.stderr`.

`cargo xtask site` assembles the catalog and mdBook with existing coverage and benchmark reports under `public`. CI creates those reports before uploading one Pages artifact.

## Releases

Increment `[workspace.package].version` for every push. Patch versions repair compatible behavior, minor versions add compatible lint capabilities, and major versions remove or change public workflows. For pre-1.0 development, a minor increment marks breaking changes. This removal of the portable runner uses 0.2.0.

CI tags and publishes a release only after tests, Clippy, self-lint, coverage, benchmarks, documentation and Nix checks pass. `cargo xtask release` validates and prints the shared version. `cargo xtask release --publish` creates the annotated tag and GitHub release. Existing tags cannot move to another commit. Major and minor maintenance branches use `release/MAJOR` and `release/MAJOR.MINOR`; existing branches are never retargeted automatically.

Creation uses an empty expected-value Git lease, so a branch created after discovery cannot move during publication. A rejected creation stops publication; retry after inspecting the remote branch.

Publication requires a stable version without prerelease or build identifiers. Version parsing preserves semver errors. Increment checks run before publication side effects.

Use the configured user identity for commits and tags. Keep release fixes on their maintenance branch and increment that branch's patch version. Git-loaded compiler libraries remain unpublished on crates.io.
