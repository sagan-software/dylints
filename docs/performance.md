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

Dylint's upstream CI caches Cargo tools, registry and Git sources, compiler toolchains and Dylint drivers. The workflow also caches workspace build artifacts. Compiler and tool caches use their own versions rather than the workspace lockfile, so workspace release bumps do not invalidate them. Restore the compiler before invoking Rustup, and skip tool installation on an exact tool-cache hit. Compiler and tool caches are saved immediately after successful setup, even if a later gate fails. Registry and Git-source caching belongs to the workspace Cargo cache to avoid uploading the same data twice. Failed main-branch validations also retain dependency artifacts, so fixing a late gate does not force another cold dependency build. Keep coverage instrumentation outputs separate from ordinary build artifacts. After uploading coverage reports, CI discards the instrumented build, raw profiles and retained objects; the next measurement deliberately rebuilds those, so caching them would waste transfer time and disk space.

CI runs ordinary tests once with nextest, with doctests and instrumented coverage as separate gates. The small xtask integration suite also runs before workspace Clippy and UI tests so broken development commands fail early. Nix verification retains its independent sandboxed test run. The previous failing run spent 3½ minutes reinstalling tools despite restoring a cache; cache-hit savings and total CI times must be measured on successful runs of the updated workflow.

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

## Clap source snapshots

Clap derive and documentation recovery now borrows rustc's retained source text
instead of reopening and reading the complete file for each lookup. The shared
source-map handle is cloned without cloning its text, and only the selected
source segment is copied. This also keeps byte offsets aligned with the compiler's
source snapshot. Files without retained text still use the existing disk fallback.

Regression tests verify that retained text, including an empty snapshot, works
without an accessible file, and that fallback reads and missing-file handling
remain available. This removes file I/O and full-file allocations on the retained
source path; no whole-suite runtime percentage has been measured for this change.

## Candidate-only public-type scanning

`unnecessary-public-type` skips workspace scanning when the crate has no
candidate types. Otherwise it stores only candidate-name counters, capped at two
occurrences, and stops reading additional files when all candidates have a second
occurrence. Identifier slices are borrowed instead of allocated per token.
The existing lexical matching policy is unchanged.

A development-profile microbenchmark scanning 50,000 unrelated unique identifiers
and two occurrences of one candidate retained 1 counter instead of 50,001.
Median scanning time over seven batches was 52.9 ms before and 20.1 ms afterward.
This excludes workspace discovery and file I/O; memory growth now follows the
number of candidate names instead of all distinct workspace identifiers.

## Shared cfg source text

`repeated-cfg-gate` now retains shared handles to rustc's source strings instead
of copying all loaded source text into a second collection. Files without `#`
are omitted before parsing because they cannot contain cfg attributes. Files
with attribute-like text still use the existing syntax-aware parser. The change
removes full-source copies and parser work for attribute-free files; it does not
change cfg classification or diagnostic locations. No whole-suite speedup is
claimed for this change.

## Coverage source identity caching

The coverage-based complexity lint resolves each reported source path once per
compilation instead of canonicalizing it for every function. Successful and
failed resolutions are cached in the report's state; caches do not persist across
compilations. Coverage still comes only from the measured executable-line records.

A development-profile microbenchmark of 5,000 coverage queries for one source file
had median batch times of 49.1 ms before and 2.1 ms after caching, over seven
batches. The report was parsed before timing. This measures repeated source lookup
and fraction calculation, not report loading, complexity analysis, or compiler
startup. It does not imply the same benefit for one-function-per-file workloads.

## Bounded catalog discovery

Catalog discovery visits only `crates/<crate>/ui` and `crates/<crate>/src`, rather
than recursively traversing fixture and build-output subtrees first. Source files
inside a selected crate's `src` are still read recursively. On the measured
checkout, one discovery walk visited 1,647 entries instead of 3,169. Seven-run
median times were 9.27 ms and 8.21 ms; the small timing difference is not evidence
of a substantial catalog-generation speedup. The structural benefit is bounded
traversal even when nested fixture/build directories grow.

## Reqwest fixture features

Reqwest fixtures explicitly enable TLS, blocking requests, cookies, and multipart
instead of enabling all defaults. Charset decoding, HTTP/2, and system-proxy
integration are not used by these compile-time fixtures. The Linux Reqwest
subtree drops from 131 to 127 distinct package/version entries: `encoding_rs`,
`fnv`, `h2`, and `tokio-util` leave that subtree. The lockfile also drops unused
platform-specific proxy dependencies. Packages required elsewhere can remain in
the workspace graph; these counts are not a measurement of total disk savings.
TLS diagnostics still compile against the existing default TLS implementation.

## Router-only Axum fixtures

Axum fixture dependencies disable default features because the fixtures exercise
routing and middleware APIs rather than server startup, JSON/form/query extractors,
or tracing integration. The Linux Axum dependency subtree drops from 64 to 35
distinct package/version entries under the measured workspace resolution. Other
workspace packages may still need some of those dependencies. All Axum UI fixtures
retain their expected diagnostics with the smaller feature configuration.

## Sparse field-cohesion graphs

The field-cohesion lint now connects each field's users to one representative
method and indexes direct callees, rather than comparing all method pairs and
materializing shared-field cliques. Connected components—and therefore the
existing diagnostic thresholds and summaries—remain the same. Regression tests
compare components against the original pairwise algorithm across 4,096 field
and call combinations, including self-calls and unknown callees.

For 300 methods sharing four fields, a development-profile microbenchmark of graph
construction and component traversal stored 598 adjacency entries instead of
89,700. Seven-run median times were 74.64 ms before and 1.67 ms afterward. This
isolates graph work; it excludes rustc traversal and does not represent a
whole-suite speedup. Sparse graphs avoid quadratic edge storage for shared fields.
