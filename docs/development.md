# Contributing

Run Cargo commands inside `nix develop`. Keep dependency sources in `[workspace.dependencies]`; consumers inherit sources and specify their own features. The virtual workspace contains category libraries, leaf lints, `support`, `web` and `xtask`. There is no portable runner.

## Validation

```sh
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --lib --bins --tests -- -D warnings
cargo test --workspace --lib --bins --tests
cargo xtask lint
cargo xtask coverage --min-lines 97 -- --workspace --lib --bins --tests
cargo xtask bench
cargo xtask site
nix fmt
nix flake check
```

Before changing a lint, add a failing UI regression at its compiler boundary. Run `cargo test -p LINT_NAME --lib` after each coherent change. When diagnostics change, copy the test's reported actual stderr into the expected fixture. When fixture imports or suggested fixes change, copy the test's reported actual fixed output into the rustfix fixture. When a fixture deliberately violates a lint, preserve that input instead of applying its suggested fix. Verify triggering and non-triggering cases.

`cargo xtask lint` checks ordinary library, binary and integration-test targets. Deliberate UI and example fixtures are excluded. New production paths require direct tests; line hits alone do not prove correctness.

Coverage tests retain artifacts after test failure and reject unsafe output targets. They propagate LLVM export, summary-write and terminal-write failures. Site tests propagate catalog and book failures before report assembly.

The process fixture is a Cargo example with `clap` derive parsing. Tests build it in a private target directory. `cargo fmt --all` includes its source; `nix fmt` checks it with the rest of the repository.

## Adding a lint

Place a lint under `lints/CATEGORY/LINT_NAME` or a crate-specific group. Its manifest inherits dependencies, package metadata and lint configuration. Declare its default level as `Warn`. Register it through its parent group's `rlib` constituent dependency. Parent groups register nested groups; discovery lists only parents.

The optional aggregate owns all nine categories and must be discovered independently. Its UI fixture also records Cargo-policy diagnostics from the isolated compiler input.

Each lint owns `Cargo.toml`, `README.md`, `src/lib.rs`, `ui/main.rs` and `ui/main.stderr`. Keep the five required README sections in order: `What it does`, `Why is this bad?`, `Known problems`, `Example` and `Use instead`. Add interpretation and primary sources after those sections when needed.

Prefer resolved compiler types, methods and definitions for semantic checks. Source-layout lints can use AST structure when they explain that boundary. Shared compiler helpers live in `support`; category helpers remain with their category.

The compiler-sysroot bootstrap tests compile and execute the unchanged Cargo build script. They verify compiler selection, symlinks, PATH order, missing configuration, invalid layouts and protocol-write failure. Instrumented runs retain this executable in the coverage object archive, because Cargo build-script binaries are excluded from ordinary object discovery.

## Reports

`cargo xtask coverage` validates percentages and scope directories before replacing generated outputs. It instruments test binaries and lint libraries, merges profiles and writes LLVM HTML plus canonical executable-line JSON, TSV and gap reports. Source aliases merge by canonical path and line, retaining maximum hits. Raw LLVM mappings remain separate. Inspect `canonical_gaps.txt` for exact zero-hit source lines; inspect assertions before treating a covered line as correct. Examples, UI and fixture sources are excluded; `cfg(test)` and ordinary integration tests are included.

Constituent builds can replace toolchain-named lint libraries with another feature variant. A Unix linker wrapper retains each successfully linked shared library before replacement. It delegates unchanged arguments to `dylint-link`; compiler commands remain visible to Dylint's UI tests. Canonical coverage exports each retained object separately and merges line hits because combined LLVM mappings can discard another variant's counters. Raw LLVM HTML and JSON retain that mapping limitation. Retaining libraries increases coverage disk use; coverage replaces only its owned build, object, profile and report directories.

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
