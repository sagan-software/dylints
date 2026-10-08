# Performance

Measure each independently loadable leaf lint against compiler workloads. Category and aggregate libraries register their constituents and are excluded from the per-lint inventory. The harness verifies discovery against Cargo metadata and uses a shared workload generator.

## Benchmarking

```sh
cargo xtask bench
cargo xtask bench --full
```

The default command performs a bounded inventory sweep: it samples every leaf lint with a prebuilt driver and small workloads using shortened warm-up and measurement windows. It retains Criterion's normal sample collection and plots. Fast and full file-size results use separate Criterion groups, so their sampling modes are not compared as code changes. Use `--full` for longer statistical measurements before comparing a performance change. Reports include compiler startup and workload checking; they do not isolate only callback time.

Use the same pinned compiler, build profile, machine and workload sizes when comparing results. Keep external workload and toolchain changes separate from the measured implementation. Open `target/criterion/report/index.html`; the site publishes this index under `benches/report/`. Keep the generated directory structure so the index can resolve its chart and result links.

Build timings are omitted. Reliable comparisons need explicitly controlled warm and cold caches, dependency state, profile and host; this report measures compiler analysis with a prebuilt driver. Timings from an uncontrolled runner would mostly describe cache state.

## Build costs

The Dylint libraries use compiler-private dependencies. Link them through `dylint-link`. Cargo's build and target directories must agree because UI tests load the linked dynamic libraries from the target directory. The Nix shell clears an inherited shared build directory so each Dylint child can select its own target directory. A forced shared build directory places linker-named libraries outside Dylint’s lookup directory.

Development and CI builds disable incremental compilation and debug information to limit artifacts. For source-level backtraces, opt in with `CARGO_PROFILE_DEV_DEBUG=line-tables-only`; use `CARGO_PROFILE_DEV_DEBUG=2` for full debugger information. These overrides rebuild affected artifacts, so use them for focused lint development. Existing artifacts from previous profiles remain on disk until explicitly cleaned.

Lint crates retain both `cdylib` and `rlib` outputs: Dylint loads the shared library, while category and aggregate crates link their constituents through Rust libraries. The `rlib` feature controls Dylint registration symbols, not Cargo's output types. Helper crates already produce only `rlib`. Removing either output from a constituent breaks standalone loading or group composition.

Nix scopes the lint dependency cache to the aggregate lint library instead of the entire workspace. Workspace verification still covers all of its original targets. The catalog is generated through the existing xtask command.

## Incremental optimization measurements

Local x86_64 Linux measurements on the 0.2.0 workspace layout with `nightly-2026-07-15` (before the 0.3 layout refactor):

| Workload | Before | After |
| --- | ---: | ---: |
| Fresh `cargo build -p perf --lib` target directory | 336.2 MiB | 145.3 MiB |
| Loadable libraries in that build | 51.4 MiB | 5.9 MiB |
| `bevy-asset-source-after-asset-plugin` test executable | 71.1 MiB | 34.0 MiB |
| Compiler check of 500 locally allowed Markdown doc blocks | 492 ms | 55 ms |

The build comparison changes only development debug information from
`line-tables-only` to `0`, using separate empty target/build directories and
already downloaded dependencies. Allocated disk usage counts each inode once.
One build of each profile took 17.0 and 15.7 seconds respectively; these are
observations, not statistically established build-time improvements. Test
executable sizes compare the same package and dependencies under both profiles.

The Markdown comparison uses identical no-debug profiles with and without the
per-item lint-level guard. Each of 500 public functions has a locally allowed
doc comment containing spaced emphasis. The lint remains enabled at crate
level, and `rumdl.toml` enables MD037. The figures are medians of five alternating
driver runs after warmup, including compiler startup and metadata emission.
This measures skipped work on suppressed documentation, not a speedup for
enabled Markdown checks or the entire lint suite. The UI regression checks
local re-enabling, local suppression, and fulfilled expectations.

These measurements exclude toolchain installation, Nix store closures, and
downstream project dependencies. Existing caches are not automatically removed.

## Parser dependencies in focused builds

Workspace lints disable `dylint-support`'s default features. Only
`large-rust-file` and `unnecessary-module-directory` enable its `rust-file-size`
feature, which supplies the shared source-size parser. Direct consumers of the
support crate retain that feature by default. This keeps unrelated lint builds
from compiling the support crate's full `syn` parser and span-location support.
Other dependencies can still require `syn`, including compiler-side proc macros.

On the 0.3.2 workspace with the pinned compiler and debug information disabled,
a fresh `cargo build --locked --offline -p perf --lib` used 145.3 MiB before
this feature split and 126.4 MiB afterward, a 13% reduction. Both measurements
used empty temporary target/build directories, warm downloaded dependencies,
and one Cargo build job; allocated disk usage counted each inode once. The
single builds took 29.1 and 25.2 seconds, respectively, which is insufficient to
establish a repeatable build-time improvement. Temporary artifacts were removed.

These savings apply to focused builds that do not enable the parser elsewhere.
The full aggregate and workspace test builds still include the parser-dependent
lints, so they are not expected to see the same reduction.

## Fixture dependencies

Bevy 0.19 fixture dependencies use umbrella re-exports and only required features. Bevy 0.18 remains for deliberate cross-version diagnostics. Bevy derive macros require the current umbrella dependency's canonical name `bevy`; aliases are not discovered by its macro manifest helper.

Dylint's upstream CI caches Cargo tools, registry and Git sources, compiler toolchains and Dylint drivers. The workflow also caches workspace build artifacts. Keep coverage instrumentation outputs separate from ordinary build artifacts.

Source: [Dylint 6.0.3 CI](https://github.com/trailofbits/dylint/blob/v6.0.3/.github/workflows/ci.yml) and [Bevy 0.19 macro manifest lookup](https://docs.rs/bevy_macro_utils/0.19.0/src/bevy_macro_utils/bevy_manifest.rs.html).

## Attribute-free source-size checks

Source-size checks skip test-region parsing when a file contains no `#` character:
every supported test-only marker requires an attribute. Sources containing `#`
still use the full parser, including comments and strings that merely resemble
attributes. Both the shared file-size helper and crate-size lint use this filter.

A development-profile microbenchmark of the shared file-size helper on 1,600
`const _: () = ();` lines took 22.65 ms before and 0.16 ms afterward per call.
These are medians of seven batches of 30 calls using `black_box`, on the pinned
compiler with debug information disabled. This isolates the helper and does not
measure compiler startup, file I/O, or full-suite runtime.

## Repeated-cfg diagnostic locations

`repeated-cfg-gate` indexes source line starts once per file that contains cfg
gates. Each diagnostic endpoint then uses an indexed lookup instead of scanning
all preceding lines. UTF-8 boundary checks, empty trailing lines, and invalid
location rejection are preserved. The index uses one `usize` per source line.

A development-profile microbenchmark resolving column zero on each of 2,000
lines took 65.49 ms before and 0.18 ms afterward, including index construction.
The figures are medians of seven batches, with inputs and results passed through
`black_box`. This measures location conversion alone, not syntax parsing or a
complete lint run; files with few gates have less opportunity to benefit.
