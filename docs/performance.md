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

Development builds disable incremental compilation and retain line-table debug information. CI disables debug information to limit artifacts. Bevy 0.19 fixture dependencies use umbrella re-exports and only required features. Bevy 0.18 remains for deliberate cross-version diagnostics. Bevy derive macros require the current umbrella dependency's canonical name `bevy`; aliases are not discovered by its macro manifest helper.

Dylint's upstream CI caches Cargo tools, registry and Git sources, compiler toolchains and Dylint drivers. The workflow also caches workspace build artifacts. Keep coverage instrumentation outputs separate from ordinary build artifacts.

Source: [Dylint 6.0.3 CI](https://github.com/trailofbits/dylint/blob/v6.0.3/.github/workflows/ci.yml) and [Bevy 0.19 macro manifest lookup](https://docs.rs/bevy_macro_utils/0.19.0/src/bevy_macro_utils/bevy_manifest.rs.html).
