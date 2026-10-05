# Benchmarks

Criterion benchmarks discover leaf lint libraries from Cargo metadata.
All 295 libraries have individual compiler-process benchmarks and completed
benchmark runs. Named lint benchmark coverage is 295/295, or 100%.
Triggering-case coverage and branch coverage require additional workloads,
particularly for crate-specific APIs.

## Run

Run the full lint inventory sweep inside the pinned development shell:

```sh
nix develop --command env SAGAN_BENCH_FAST_SWEEP=1 \
  cargo bench -p sagan-lints --bench lint_passes --profile dev -- --quick --noplot
```

This runs all 295 named lint cases, the compiler controls, and the six
`file_size` cases. It writes compiler results under `compiler_fast/` so they
remain separate from standard workload results. Each named lint compiles one
documented function. The all-lints case compiles four modules with eight
functions each. Criterion's quick mode and 10 ms measurement ceiling keep the
run short. The command skips plot and HTML generation while keeping the estimates.

The sweep uses 1,000 bootstrap resamples instead of the standard
10,000. Its estimates do not support statistical confidence claims.
One verified run took 13.193 seconds with all `target/lint-bench` libraries already
built. A cold library build adds setup time and may exceed this limit.

The standard workload uses the original sizes and longer Criterion samples:

```sh
nix develop --command cargo bench -p sagan-lints --bench lint_passes --profile dev
```

The harness builds leaf libraries in `target/lint-bench/` before timing.
This isolates them from category builds, whose `constituent` feature removes
individual library entry points. The development profile uses unoptimized
lint libraries.

Each compiler iteration starts the pinned Dylint driver, loads libraries,
checks source, processes diagnostics, and writes metadata. Setup, Cargo
metadata discovery, library compilation, and source generation are untimed.
Analysis workloads use an empty `DYLINT_TOML` override. A separate
`compiler/config_discovery` workload retains default discovery.

The standard individual workload has one module with 32 functions. The combined
`compiler/all_lints` workload has eight modules with 128 functions each.
`compiler/plain` provides a compiler-only control. The `file_size` group
checks repository source files, empty input, and exact line-limit boundaries.

The optional `repository/lint` case runs only when `SAGAN_BENCH_RUNNER` is set.
It clears workspace artifacts before each sample and measures both repository
lint passes, so it is excluded from the under-one-minute inventory sweep.

## Repository comparison

`repository/lint` measures this repository through the bundled runner.
Each sample runs the default-target check, then the test-target check.
Both runs use strict Clippy and private lints with repository configuration,
matching the two commands in the Nix self-lint check.

Workspace artifacts are cleared before each sample. Dependency artifacts
remain in `target/repository-bench/`. Cleanup and benchmark setup are untimed.
The timer includes both runner invocations and any dependency rechecking
Cargo requires. Failed checks reject the sample.

Build and retain separate before and after runner executables. Use one frozen
source tree for both executables. Set `SAGAN_BENCH_RUNNER` to the executable
and `SAGAN_BENCH_REPOSITORY` to that source tree:

```sh
nix develop --command env \
  SAGAN_BENCH_RUNNER=/tmp/runner-before \
  SAGAN_BENCH_REPOSITORY=/tmp/lint-input \
  cargo bench -p sagan-lints --bench lint_passes --profile dev -- \
  repository/lint --save-baseline before

nix develop --command env \
  SAGAN_BENCH_RUNNER=/tmp/runner-after \
  SAGAN_BENCH_REPOSITORY=/tmp/lint-input \
  cargo bench -p sagan-lints --bench lint_passes --profile dev -- \
  repository/lint --baseline before
```

Keep the source, toolchain, profile, flags, and target directory identical.
Before collecting comparisons, allow the dependency cache to warm.
Criterion stores results under `target/criterion/`.
Individual compiler times include startup and cannot be summed to estimate
repository lint time.

## Results

Measurements used the same archived commit
`ba8c419d22ea79ce11499d7d10f6e153dc68e964`, including the README agent's changes.
Both executables used the pinned Rust 1.99 nightly from 2026-07-15 and the
development profile on an Intel i7-8565U T490 with 31 GiB RAM.
The dependency warm-up and failed setup runs were excluded.

Three paired runs recorded these overall times:

- 697.90 s before, 443.31 s after: 36.48% reduction.
- 674.31 s before, 609.30 s after: 9.64% reduction.
- 708.27 s before, 577.57 s after: 18.45% reduction.

Mean overall time fell from 693.49 s to 543.39 s, a 21.64% reduction.
In the final pair, Clippy changed from 394.21 s to 386.43 s, or 1.97%.
Private lint time fell from 313.98 s to 190.90 s, or 39.20%.
The final pair exceeded the 10% overall target with a nearly unchanged control.

Other agent compilation continued on this shared host. These are pilot
measurements, including one pair below the target.
For long iterations, Criterion's `--quick` mode duplicates one measured value.
Its confidence intervals and p-values cannot establish statistical confidence
from these samples. Use the default ten samples on a quiet host for that claim.

Raw Criterion data remains under `target/criterion/repository/lint/`.
See Criterion's [sampling implementation](https://docs.rs/crate/criterion/0.8.2/source/src/routine.rs).

The runner resolves workspace configuration once with `cargo locate-project`
and passes its unchanged text through Dylint 6.0.3's `DYLINT_TOML` override.
Inherited overrides, custom Cargo commands, unsupported extra Cargo arguments,
and an inherited `RUSTC_WRAPPER` retain upstream discovery.
Missing or unreadable files, invalid UTF-8, NUL bytes, and configurations larger
than 8 KiB also retain upstream discovery.
Tests verify cached configuration changes, malformed input, override precedence,
and external dependency isolation.

The shared file-size policy parses test regions only for 1,500–1,999-line files.
Other line counts determine the result directly. Boundary tests compare
production, test-heavy, and malformed sources with the original calculation.
Pilot measurements for 1,499-line input fell from 168.26 ms to 2.23 ms.
The 2,000-line case fell from 260.94 ms to 0.63 ms. Host load varied, including
on the unchanged 1,500-line path, so these are illustrative measurements.

The scoped coverage report records hits on every changed executable
optimization line and all benchmark setup helper lines. Unchanged gaps remain
at `src/runtime.rs:53` and `support/dylint/src/rust_file_size.rs:197,226`.
LLVM reports `warning: 3 functions have mismatched data`. The benchmark entry-point
control and panic paths were not line-instrumented; timed execution coverage
and instrumented line coverage are separate measurements.

Criterion 0.8.2 controls timing and comparisons. See its
[API documentation](https://docs.rs/criterion/0.8.2/criterion/) and
[timing-loop guide](https://bheisler.github.io/criterion.rs/book/user_guide/timing_loops.html).
