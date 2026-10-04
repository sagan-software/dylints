# Setup and use

[Back to the README](../README.md)

Start with the [quick start](../README.md#quick-start). This guide covers other
ways to select and run the lints. The configuration format follows
[Dylint 6.0.3](https://github.com/trailofbits/dylint/tree/v6.0.3#workspace-metadata),
the version used by this repository.

## Choose lint libraries

Dylint reads library entries from your workspace's root `Cargo.toml` or
`dylint.toml`. The entries work for a single-crate project too. Add each entry
once, in one file. Keep existing entries when adding these lints.

Use `git` for the source repository, `rev` for a fixed commit, and `pattern`
for library directories. Paths are relative to the lint repository, not your
project. These settings do not belong under `[dependencies]`.

### All groups

To load all nine top-level groups, use:

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
rev = "483b64d83e38352994d509eacf4a56db1892f1a3"
pattern = "lints/*"
```

This includes `restriction` and project policy checks. Review the
[groups](../README.md#lint-groups) and their rules before enabling them all.
Select top-level groups or individual libraries. Loading a group alongside
one of its own lints can register the same lint twice.

### One lint

To load only `ownership_at_boundaries`, use:

```toml
[[workspace.metadata.dylint.libraries]]
git = "https://github.com/sagan-software/dylints"
rev = "483b64d83e38352994d509eacf4a56db1892f1a3"
pattern = "lints/perf/ownership_at_boundaries"
```

For crate-specific checks, you can select a crate group such as
`lints/crates/serde` or an individual lint beneath it. Find paths in the
[`lints` directory](../lints).

### Local checkout

For local lint development, replace the Git entry with a path entry:

```toml
[[workspace.metadata.dylint.libraries]]
path = "../dylints/lints/perf"
```

The path is relative to your project root. Run Cargo commands in this
repository's `nix develop` shell when building or testing its lint source.

### Update the pin

The examples pin a commit to keep lint behavior and the compiler version
repeatable. Before changing `rev`, read the chosen revision's
[`rust-toolchain.toml`](../rust-toolchain.toml) and Dylint version in
[`flake.nix`](../flake.nix). Install those versions, update the pin, and run
checks locally before updating CI.

You can replace `rev` with `branch = "main"` to follow development. A branch
can change the rules and required compiler between runs. Use a commit pin for
repeatable team and CI checks.

## Run and inspect

Use the linker setup from the quick start in each terminal.
Run from your project root:

```sh
cargo dylint --all --workspace -- --all-targets
```

List the checks in the selected libraries:

```sh
cargo dylint list --all
```

Run just the configured `perf` library on one workspace package:

```sh
cargo dylint --lib perf --package my_crate -- --all-targets
```

Replace `my_crate` with a package name from your project's `Cargo.toml`.
`--lib perf` requires an entry that selects `lints/perf`.
Arguments after `--` go to `cargo check`, such as `--all-features`.
Dylint compiles code to check it; it does not run the project's tests.

## Automatic fixes

Before applying fixes, commit or save your work so you can review the edits.
Run:

```sh
cargo dylint --fix --all --workspace -- --all-targets
```

Only lints with machine-applicable suggestions can provide automatic fixes.
After applying fixes, review the diff and run your project's tests.

## Allow one lint

For a lint loaded through the `perf` group, use a conditional attribute:

```rust
#[cfg_attr(dylint_lib = "perf", allow(ownership_at_boundaries))]
pub fn count_lines(lines: Vec<String>) -> usize {
    lines.len()
}
```

The attribute names the loaded library and the lint separately. If you load
only the individual lint, use `dylint_lib = "ownership_at_boundaries"` instead.
This avoids unknown-lint warnings during ordinary builds.

Declare the custom configuration key in a single crate's `Cargo.toml`:

```toml
[lints.rust.unexpected_cfgs]
level = "warn"
check-cfg = ['cfg(dylint_lib, values("perf"))']
```

For a workspace, put the rule under
`[workspace.lints.rust.unexpected_cfgs]` and enable `[lints] workspace = true`
in each member. If your project already defines these tables, extend them.
Add each library name you use to `values(...)`.

Dylint's
[conditional compilation guide](https://github.com/trailofbits/dylint/tree/v6.0.3#conditional-compilation)
explains the special case for checks that run before macros expand.

## CI

Use the same lint pin, Dylint version, and compiler locally and in CI.
On a Linux runner with Rust and a native linker installed, run:

```sh
rustup toolchain install nightly-2026-07-15 --component rustc-dev --component llvm-tools-preview
cargo +nightly-2026-07-15 install --locked --version 6.0.3 cargo-dylint dylint-link
unset CARGO_BUILD_BUILD_DIR
lint_host=$(rustc +nightly-2026-07-15 -vV | sed -n 's/^host: //p')
lint_host=$(printf '%s' "$lint_host" | tr '[:lower:]-' '[:upper:]_')
export "CARGO_TARGET_${lint_host}_LINKER=dylint-link"
DYLINT_RUSTFLAGS="-D warnings" cargo dylint --all --workspace -- --all-targets
```

The last command fails on lint warnings and other compiler warnings.
Run `cargo test` and `cargo clippy` as separate checks with your project's
usual toolchain. Library builds need network access on the first run.
`cargo dylint --help` describes `DYLINT_DRIVER_PATH` and
`DYLINT_LIBRARY_PATH` for cache setup.

## Editor checks

For VS Code with rust-analyzer, add this to `.vscode/settings.json`:

```json
{
  "rust-analyzer.check.overrideCommand": [
    "cargo", "dylint", "--all", "--workspace", "--",
    "--all-targets", "--message-format=json"
  ]
}
```

This replaces rust-analyzer's check command with Dylint. Run Clippy separately
if you also want its diagnostics. See
[Dylint's editor guide](https://github.com/trailofbits/dylint/tree/v6.0.3#vs-code-integration).

## Bundled runner

The `sagan-lints` runner combines strict Clippy with its embedded lint groups.
Unlike direct Dylint use, it skips some checks that require repository policy
files unless you pass `--include-repo-policy-lints`.

With Nix and flakes enabled, run:

```sh
nix run github:sagan-software/dylints -- --repo /path/to/your/project --fast
```

Replace `/path/to/your/project` with your project's directory. To pin the Nix
package too, use `github:sagan-software/dylints/483b64d83e38352994d509eacf4a56db1892f1a3`.

Use one package, one group, or automatic fixes:

```sh
nix run github:sagan-software/dylints -- --repo /path/to/your/project --package my_crate
nix run github:sagan-software/dylints -- \
  --repo /path/to/your/project --skip-clippy --dylint-category perf
nix run github:sagan-software/dylints -- --repo /path/to/your/project --fix
```

Before using `--fix`, save your work. After using `--fix`, review the diff.
After using `--fix`, run your project's tests.
Run `nix run github:sagan-software/dylints -- --help`
for all options, including changed-line checks and log paths.

## Setup errors

### No libraries found

Check that the library entry is in the workspace root and that its pattern
matches a library directory. Run `cargo dylint list --all` from that root.
Confirm that `cargo-dylint` and `dylint-link` are both on `PATH`.

### Missing compiler components

Install the compiler components shown in the quick start. These libraries
need `rustc-dev`. Dylint needs the compiler used to build the libraries,
as described in its
[toolchain limits](https://github.com/trailofbits/dylint/blob/v6.0.3/docs/how_dylint_works.md#limitations).

### Project needs a newer compiler

Dylint uses the lint library's compiler to check your project. If your project
or a dependency requires a newer compiler, this pinned revision cannot check
it. Choose a lint revision built for a compatible compiler. Changing only
your project's toolchain does not update the lint libraries.

### Linker or native dependency errors

Install your platform's native compiler and linker. If your project needs
system libraries, build scripts, or environment variables, provide those as
you would for `cargo check`. Dylint also builds the project's dependencies.

The quick start selects `dylint-link` through an environment variable for
your machine's target. This repository's development shell supplies that
setting for building lint libraries and running their UI tests.

### Successful build but library not found

Dylint needs the library copy that `dylint-link` creates with a compiler
version in its filename. If Cargo uses a separate build directory through
`CARGO_BUILD_BUILD_DIR`, that copy can remain outside the directory Dylint
searches. Run `unset CARGO_BUILD_BUILD_DIR` and repeat the linker setup from
the quick start, then rerun the check.
